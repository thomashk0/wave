use std::io;
use std::io::Read;
use std::str;
use std::str::FromStr;

#[cfg(test)]
use nom::error::ErrorKind;
use nom::{
    branch::alt,
    bytes::streaming::{tag, take, take_till, take_till1},
    character::streaming::{
        alphanumeric1, char, digit1, multispace0, multispace1, none_of, one_of,
    },
    combinator::{map, map_res, opt},
    error::ParseError,
    number::streaming::recognize_float,
    sequence::{delimited, preceded, separated_pair, terminated, tuple},
    IResult,
};
use serde::Serialize;

use crate::types::{Direction, Range, Scope, VariableInfo, VariableKind};
use crate::utils;

#[derive(Debug)]
pub enum VcdError {
    IoError(io::Error),
    ParseError,
    MissingData,
    PartialHeader,
    Utf8Error,
    EndOfInput,
}

impl std::fmt::Display for VcdError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            VcdError::IoError(e) => e.fmt(f),
            x => write!(f, "{:?}", x),
        }
    }
}

impl std::error::Error for VcdError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            VcdError::IoError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<io::Error> for VcdError {
    fn from(e: io::Error) -> Self {
        VcdError::IoError(e)
    }
}

impl<'a, E: ParseError<&'a str>> From<nom::Err<E>> for VcdError {
    fn from(e: nom::Err<E>) -> Self {
        match e {
            nom::Err::Incomplete(_) => VcdError::MissingData,
            _ => VcdError::ParseError,
        }
    }
}

#[derive(Debug, Serialize, PartialEq)]
pub struct VcdChange<'a> {
    pub var_id: &'a str,
    pub value: VcdValue<'a>,
}

#[derive(Debug, Serialize, PartialEq)]
pub enum VcdValue<'a> {
    Bit(char),
    Vector(&'a str),
    Real(&'a str),
}

impl<'a> VcdValue<'a> {
    pub fn width(&self) -> usize {
        match self {
            VcdValue::Bit(_) => 1,
            VcdValue::Vector(v) => v.len(),
            VcdValue::Real(_) => 1,
        }
    }
}

#[derive(Debug, Serialize)]
pub enum VcdCommand<'a> {
    Directive(&'a str),
    VcdEnd,
    SetCycle(u64),
    ValueChange(VcdChange<'a>),
}

#[derive(Clone, Debug, Serialize)]
pub struct VcdHeader {
    pub variables: Vec<VariableInfo>,
}

pub struct VcdHeaderParser {
    pub header: VcdHeader,
    header_valid: bool,
    scope: Vec<Scope>,
    verbose: bool,
}

impl VcdHeaderParser {
    pub fn new() -> Self {
        VcdHeaderParser {
            header: VcdHeader {
                variables: Vec::with_capacity(1024),
            },
            header_valid: false,
            scope: Vec::with_capacity(16),
            verbose: false,
        }
    }

    fn next_header_command<'a, E: ParseError<&'a str>>(
        &mut self,
        input: &'a str,
    ) -> IResult<&'a str, bool, E> {
        let (remaining, cmd) = terminated(preceded(char('$'), alphanumeric1), multispace0)(input)?;
        match cmd {
            "enddefinitions" => {
                let (remaining, _) = vcd_end(remaining)?;
                self.header_valid = true;
                Ok((remaining, true))
            }
            "scope" => {
                let (remaining, (kind, name)) =
                    terminated(tuple((vcd_word, vcd_word)), vcd_end)(remaining)?;
                self.scope.push(Scope::from_str(kind, name));
                Ok((remaining, false))
            }
            "upscope" => {
                let (remaining, _) = vcd_end(remaining)?;
                self.scope.pop();
                Ok((remaining, false))
            }
            "var" => {
                let (remaining, (var_type, width, var_id, var_name, range)) =
                    terminated(
                        tuple((vcd_word, var_width, vcd_word, var_name, opt(var_range))),
                        vcd_end,
                    )(remaining)?;
                self.header.variables.push(VariableInfo {
                    id: String::from(var_id),
                    kind: VariableKind::from(var_type),
                    width: width as u32,
                    name: String::from(var_name),
                    range,
                    handle: 0,
                    scope: self.scope.clone(),
                    direction: Direction::Implicit,
                });
                Ok((remaining, false))
            }
            x => {
                if self.verbose {
                    eprintln!("warning: ignoring directive {}", x);
                }
                let (remaining, _) = skip_until_vcd_end(remaining)?;
                Ok((remaining, false))
            }
        }
    }

    pub fn header(&self) -> Option<&VcdHeader> {
        if self.header_valid {
            Some(&self.header)
        } else {
            None
        }
    }

    pub fn header_valid(&self) -> bool {
        self.header_valid
    }

    pub fn run<'a, E: ParseError<&'a str>>(&mut self, input: &'a str) -> IResult<&'a str, (), E> {
        let mut w = input;
        loop {
            let (remaining, done) = self.next_header_command(w)?;
            if done || remaining.is_empty() {
                return Ok((remaining, ()));
            }
            w = remaining;
        }
    }
}

/// This struct attempts to wrap the logic for running streaming parsers
struct VcdStreamParser<R> {
    buff: utils::Buffer<R>,
    chunk_size: usize,
    end_of_input: bool,
}

impl<R: Read> VcdStreamParser<R> {
    pub fn with_chunk_size(chunk_size: usize, inner: R) -> Self {
        VcdStreamParser {
            buff: utils::Buffer::with_capacity(2 * chunk_size, inner),
            chunk_size,
            end_of_input: false,
        }
    }

    pub fn done(&self) -> bool {
        self.end_of_input && self.buff.data().len() == 0
    }

    pub fn trim_refill(&mut self) -> Result<usize, VcdError> {
        loop {
            let n = self.buff.refill(self.chunk_size)?;
            let n_ws = self.buff.trim();
            if n_ws == 0 || n_ws < n {
                return Ok(n - n_ws);
            }
        }
    }

    /// Refills a chunk of data
    ///
    /// Returns the number of bytes read, returns 0 at the end of file
    pub fn refill(&mut self, trim: bool) -> Result<usize, VcdError> {
        let n = {
            if trim {
                self.trim_refill()
            } else {
                self.buff.refill(self.chunk_size).map_err(VcdError::from)
            }
        }?;
        if self.buff.data().iter().rev().take(n).any(|c| *c >= 128) {
            return Err(VcdError::Utf8Error);
        }
        // eprintln!("info: refilling {} bytes", n);
        if n == 0 {
            self.end_of_input = true;
            if !trim {
                self.buff.push(b'\n');
            }
        }
        Ok(n)
    }

    pub fn run_parser<T, F>(&mut self, mut f: F) -> Result<T, VcdError>
    where
        F: FnMut(&str) -> Result<(usize, T), VcdError>,
    {
        loop {
            let s = unsafe {
                // NOTE: we check on refill that any incoming data is made of **only** ASCII
                // characters, thus the unchecked conversion is safe.
                str::from_utf8_unchecked(self.buff.data())
            };
            // println!("info: buff({:3} unused) = {:?}", self.buff.unused(), s);
            match f(s) {
                Ok((n_remaining, v)) => {
                    let consumed = self.buff.len() - n_remaining;
                    self.buff.consume(consumed);
                    if self.buff.len() == 0 {
                        // We need to trim leading whitespaces between VCD commands
                        self.refill(true)?;
                    } else if !self.end_of_input && (self.buff.len() <= 256) {
                        self.buff.shift();
                        self.refill(false)?;
                    }
                    return Ok(v);
                }
                Err(VcdError::MissingData) => {
                    let n_read = self.refill(false)?;
                    if n_read == 0 && self.end_of_input {
                        return Err(VcdError::MissingData);
                    }
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }
    }
}

pub struct VcdParser<R> {
    buffer: VcdStreamParser<R>,
    header_parser: VcdHeaderParser,
}

impl<R: Read> VcdParser<R> {
    pub fn with_chunk_size(chunk_size: usize, inner: R) -> Self {
        VcdParser {
            buffer: VcdStreamParser::with_chunk_size(chunk_size, inner),
            header_parser: VcdHeaderParser::new(),
        }
    }

    pub fn load_header(&mut self) -> Result<&VcdHeader, VcdError> {
        type E<'a> = (&'a str, nom::error::ErrorKind);
        let buffer = &mut self.buffer;
        let header_parser = &mut self.header_parser;
        loop {
            let done = buffer.run_parser(|i| {
                header_parser
                    .next_header_command::<E>(i)
                    .map_err(VcdError::from)
                    .map(|(s, v)| (s.len(), v))
            })?;
            if done {
                return Ok(&self.header_parser.header);
            }
        }
    }

    pub fn header(&self) -> Option<&VcdHeader> {
        self.header_parser.header()
    }

    pub fn done(&self) -> bool {
        self.buffer.done()
    }

    pub fn process_vcd_commands<F>(&mut self, mut callback: F) -> Result<(), VcdError>
    where
        F: FnMut(VcdCommand) -> bool,
    {
        let mut should_stop = false;
        if self.buffer.buff.len() == 0 {
            let n = self.buffer.refill(true)?;
            if n == 0 {
                return Ok(());
            }
        }
        while !should_stop && !self.buffer.done() {
            self.buffer.run_parser(|i| {
                let (s, cmd) = vcd_command::<(&str, nom::error::ErrorKind)>(i)?;
                if callback(cmd) {
                    should_stop = true;
                }
                Ok((s.len(), ()))
            })?;
        }
        Ok(())
    }
}

/// Parse whitespaces between VCD commands, this parser is **complete** (i.e., it succeeds on empty
/// input)
fn fill_ws1<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    nom::character::complete::multispace1(input)
}

fn number<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i64, E> {
    let (input, c) = opt(char('-'))(input)?;
    let sign = if c.is_some() { -1 } else { 1 };
    map_res(digit1, |r| i64::from_str(r))(input).map(|(r, x)| (r, sign * x))
}

fn var_width<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, i64, E> {
    terminated(number, multispace0)(input)
}

fn var_range<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, Range, E> {
    let dual_range = map(
        separated_pair(var_width, terminated(char(':'), multispace0), var_width),
        |r| Range::Range(r),
    );
    let simple_range = map(var_width, |w| {
        assert!(w >= 0);
        Range::Bit(w as u64)
    });
    delimited(
        terminated(char('['), multispace0),
        alt((dual_range, simple_range)),
        terminated(char(']'), multispace0),
    )(input)
}

fn var_name<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &str, E> {
    none_of("$")(input)?;
    terminated(
        take_till1(|c: char| c.is_whitespace() || c == '['),
        multispace0,
    )(input)
}

/// Any non whitespace stuff inside commands
fn vcd_word<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    // FIXME: confirm that commenting this doesn't hurts
    // none_of("$")(input)?;
    terminated(take_till1(|c: char| c.is_whitespace()), multispace1)(input)
}

/// Matches a VCD $end token
fn vcd_end<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    const END_TAG: &str = "$end";
    terminated(tag(END_TAG), alt((fill_ws1, multispace1)))(input)
}

/// Ignores anything until a $end token is found
fn skip_until_vcd_end<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, (), E> {
    let mut w = input;
    loop {
        let (remaining, _) = take_till(|c: char| c == '$')(w)?;
        let (remaining, v) = opt(vcd_end)(remaining)?;
        if let Some(_) = v {
            return Ok((remaining, ()));
        }
        let (remaining, _) = take(1usize)(remaining)?;
        w = &remaining;
    }
}

fn vcd_cycle<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, u64, E> {
    map_res(delimited(char('#'), digit1, fill_ws1), |r| u64::from_str(r))(input)
}

/// Any non whitespace stuff inside commands
fn vcd_varid<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    terminated(take_till1(|c: char| c.is_whitespace()), fill_ws1)(input)
}

fn vcd_bit_change<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (char, &'a str), E> {
    tuple((one_of("01xXzZwWuU"), preceded(multispace0, vcd_varid)))(input)
}

fn is_vcd_bit(c: char) -> bool {
    return ['0', '1', 'x', 'X', 'z', 'Z', 'u', 'U', 'w', 'W'].contains(&c);
}

fn vcd_bits<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, &'a str, E> {
    terminated(take_till1(|c: char| !is_vcd_bit(c)), multispace0)(input)
}

fn vcd_vec_change<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (&'a str, &'a str), E> {
    preceded(
        char('b'),
        preceded(multispace0, tuple((vcd_bits, vcd_varid))),
    )(input)
}

fn vcd_real_change<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, (&'a str, &'a str), E> {
    preceded(
        char('r'),
        preceded(
            multispace0,
            tuple((terminated(recognize_float, multispace0), vcd_varid)),
        ),
    )(input)
}

fn vcd_change<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VcdChange<'a>, E> {
    alt((
        map(vcd_bit_change, |(c, var_id)| VcdChange {
            var_id,
            value: VcdValue::Bit(c),
        }),
        map(vcd_vec_change, |(value, var_id)| VcdChange {
            var_id,
            value: VcdValue::Vector(value),
        }),
        map(vcd_real_change, |(value, var_id)| VcdChange {
            var_id,
            value: VcdValue::Real(value),
        }),
    ))(input)
}

fn vcd_directive<'a, E: ParseError<&'a str>>(
    input: &'a str,
) -> IResult<&'a str, VcdCommand<'a>, E> {
    let (remaining, cmd) = terminated(preceded(char('$'), alphanumeric1), fill_ws1)(input)?;
    match cmd {
        "end" => Ok((remaining, VcdCommand::VcdEnd)),
        "comment" => {
            let (remaining, _) = skip_until_vcd_end(remaining)?;
            Ok((remaining, VcdCommand::Directive(cmd)))
        }
        _ => Ok((remaining, VcdCommand::Directive(cmd))),
    }
}

/// Parse the next VCD Command (i.e., stuff not in the VCD header) found in the given string
fn vcd_command<'a, E: ParseError<&'a str>>(input: &'a str) -> IResult<&'a str, VcdCommand<'a>, E> {
    alt((
        map(vcd_change, VcdCommand::ValueChange),
        map(vcd_cycle, VcdCommand::SetCycle),
        vcd_directive,
    ))(input)
}

/// Loop on VCD commands and execute a given callback for each one of them
pub fn process_vcd_commands<'a, E: ParseError<&'a str>, F>(
    input: &'a str,
    mut callback: F,
) -> IResult<&'a str, (), E>
where
    F: FnMut(VcdCommand) -> bool,
{
    let mut w = input;
    loop {
        let (remaining, cmd) = vcd_command(w)?;
        w = remaining;
        if callback(cmd) {
            return Ok((w, ()));
        }
        if remaining.is_empty() {
            break;
        }
    }
    Ok((w, ()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_var_width() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(var_width::<E>("1209   ..."), Ok(("...", 1209)));
        assert_eq!(var_width::<E>("3\n\t   ..."), Ok(("...", 3)));
        assert_eq!(var_width::<E>("43xx "), Ok(("xx ", 43)));
        assert_eq!(var_width::<E>("1 a"), Ok(("a", 1)));
        // Cannot fit in an u64
        assert!(var_width::<E>("184467440737095516160000").is_err());
        assert!(var_width::<E>(" 3").is_err());
    }

    #[test]
    fn test_var_range() {
        type E<'a> = (&'a str, ErrorKind);
        for v in ["[ 4  ]  ...", "[4 ]\n...", "[4]\t..."].iter() {
            assert_eq!(var_range::<E>(v), Ok(("...", Range::Bit(4))));
        }
        let w = [
            "[12:0]xx",
            "[ 12:0]\nxx",
            "[12 :0]\nxx",
            "[12 : 0]\nxx",
            "[ 12 : 0 ]\nxx",
        ];
        for v in w.iter() {
            assert_eq!(var_range::<E>(v), Ok(("xx", Range::Range((12, 0)))));
        }
        assert_eq!(
            var_range::<E>("[-1: 0] xx"),
            Ok(("xx", Range::Range((-1, 0))))
        );
    }

    #[test]
    fn test_vcd_end() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(vcd_end::<E>("$end "), Ok(("", "$end")));
        assert_eq!(vcd_end::<E>("$end \nab"), Ok(("ab", "$end")));
        assert!(vcd_end::<E>("$enddefinition \nab").is_err());
    }

    #[test]
    fn test_var_name() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(var_name::<E>("foo \nab"), Ok(("ab", "foo")));
        assert_eq!(var_name::<E>("foo[7] \nab"), Ok(("[7] \nab", "foo")));
        assert!(var_name::<E>("$foo[7] \nab").is_err());
        assert!(var_name::<E>(" foo[7] \nab").is_err());
        assert!(var_name::<E>("[foo[7] \nab").is_err());
    }

    #[test]
    fn test_vcd_word() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(vcd_word::<E>("foo $xxx "), Ok(("$xxx ", "foo")));
        assert_eq!(vcd_word::<E>("$foo aa"), Ok(("aa", "$foo")));
    }

    #[test]
    fn test_skip_until_end() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(skip_until_vcd_end::<E>("foo$hello $end "), Ok(("", ())));
        assert_eq!(
            skip_until_vcd_end::<E>("body \n\n hello $date $end \t x"),
            Ok(("x", ()))
        );
    }

    #[test]
    fn test_vcd_cycle() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(vcd_cycle::<E>("#1244 $end"), Ok(("$end", 1244)));
        assert_eq!(vcd_cycle::<E>("#123456789 "), Ok(("", 123456789)));
        assert!(vcd_cycle::<E>("#bla $end").is_err());
        assert!(vcd_cycle::<E>("# 12 $end").is_err());
    }

    #[test]
    fn test_vcd_change() {
        type E<'a> = (&'a str, ErrorKind);
        assert_eq!(vcd_bit_change::<E>("x!! #2"), Ok(("#2", ('x', "!!"))));
        assert_eq!(
            vcd_bit_change::<E>("1 hhhxr' 0"),
            Ok(("0", ('1', "hhhxr'")))
        );
        assert_eq!(vcd_vec_change::<E>("b1 x "), Ok(("", ("1", "x"))));
        assert_eq!(
            vcd_vec_change::<E>("b1001101 lala "),
            Ok(("", ("1001101", "lala")))
        );
        assert_eq!(
            vcd_vec_change::<E>("bZzXxUu01 vid ..."),
            Ok(("...", ("ZzXxUu01", "vid")))
        );
        assert_eq!(
            vcd_real_change::<E>("r3.22 # oups"),
            Ok(("oups", ("3.22", "#")))
        );
        assert_eq!(
            vcd_change::<E>("b01110 ! "), // TODO: support without space
            Ok((
                "",
                VcdChange {
                    var_id: "!",
                    value: VcdValue::Vector("01110"),
                }
            ))
        );
    }
}
