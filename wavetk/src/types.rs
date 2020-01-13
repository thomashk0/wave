use serde::Serialize;
use std::convert::TryFrom;

#[derive(Clone, Debug, Serialize, PartialEq)]
pub enum Range {
    Bit(u64),
    Range((i64, i64)),
}

/// For enums represented by an integer type, this macro implements the
/// TryFrom trait. The conversion is done by a direct std::mem::transmute
/// (unsafe), but the value is checked to be less than Type::End before
/// converting.
macro_rules! enum_direct_conversion {
    ($t:ty, $o:ty) => {
        impl TryFrom<$o> for $t {
            type Error = ();

            fn try_from(x: $o) -> Result<Self, Self::Error> {
                // Transmute would be invalid otherwise
                assert_eq!(std::mem::size_of::<$t>(), std::mem::size_of::<$o>());
                if (x >= <$t>::End as $o) {
                    Err(())
                } else {
                    let z = x as $o;
                    let r = unsafe { std::mem::transmute::<$o, $t>(z) };
                    Ok(r)
                }
            }
        }
    };
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[repr(u8)]
pub enum ScopeKind {
    VcdModule = 0,
    VcdTask = 1,
    VcdFunction = 2,
    VcdBegin = 3,
    VcdFork = 4,
    VcdGenerate = 5,
    VcdStruct = 6,
    VcdUnion = 7,
    VcdClass = 8,
    VcdInterface = 9,
    VcdPackage = 10,
    VcdProgram = 11,

    VhdlArchitecture = 12,
    VhdlProcedure = 13,
    VhdlFunction = 14,
    VhdlRecord = 15,
    VhdlProcess = 16,
    VhdlBlock = 17,
    VhdlForGenerate = 18,
    VhdlIfGenerate = 19,
    VhdlGenerate = 20,
    VhdlPackage = 21,

    Other = 22,
    End = 23,
}

enum_direct_conversion!(ScopeKind, u8);

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[repr(u8)]
pub enum VariableKind {
    VcdEvent = 0,
    VcdInteger = 1,
    VcdParameter = 2,
    VcdReal = 3,
    VcdRealParameter = 4,
    VcdReg = 5,
    VcdSupply0 = 6,
    VcdSupply1 = 7,
    VcdTime = 8,
    VcdTri = 9,
    VcdTriand = 10,
    VcdTrior = 11,
    VcdTrireg = 12,
    VcdTri0 = 13,
    VcdTri1 = 14,
    VcdWand = 15,
    VcdWire = 16,
    VcdWor = 17,
    VcdPort = 18,
    VcdSparray = 19,
    VcdRealtime = 20,

    GenString = 21,

    SvBit = 22,
    SvLogic = 23,
    SvInt = 24,
    SvShortint = 25,
    SvLongint = 26,
    SvByte = 27,
    SvEnum = 28,
    SvShortreal = 29,
    End = 30,
}

impl From<&str> for VariableKind {
    fn from(name: &str) -> Self {
        match name {
            "event" => VariableKind::VcdEvent,
            "integer" => VariableKind::VcdInteger,
            "parameter" => VariableKind::VcdParameter,
            "real" => VariableKind::VcdReal,
            "reg" => VariableKind::VcdReg,
            "supply0" => VariableKind::VcdSupply0,
            "supply1" => VariableKind::VcdSupply1,
            "time" => VariableKind::VcdTime,
            "tri" => VariableKind::VcdTri,
            "triand" => VariableKind::VcdTriand,
            "trior" => VariableKind::VcdTrior,
            "trireg" => VariableKind::VcdTrireg,
            "tri0" => VariableKind::VcdTri0,
            "tri1" => VariableKind::VcdTri1,
            "wand" => VariableKind::VcdTriand,
            "wire" => VariableKind::VcdWire,
            "wor" => VariableKind::VcdWor,
            _ => VariableKind::End,
        }
    }
}

enum_direct_conversion!(VariableKind, u8);

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
#[repr(u8)]
pub enum Direction {
    Implicit = 0,
    Input = 1,
    Output = 2,
    Inout = 3,
    Buffer = 4,
    Linkage = 5,
    End = 6,
}

enum_direct_conversion!(Direction, u8);

/// Identifiers for variables
type VarHandle = u32;

/// Analogous to VariableInfo (for VCD), the two representation will be merged soon
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct VariableInfo {
    pub id: String,
    pub name: String,
    pub direction: Direction,
    pub kind: VariableKind,
    pub width: u32,
    pub range: Option<Range>,
    pub handle: VarHandle,
    pub scope: Vec<Scope>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Scope {
    pub kind: ScopeKind,
    pub name: String,
}

impl Scope {
    pub fn from_str(kind_str: &str, name: &str) -> Self {
        let kind = match kind_str {
            "module" => ScopeKind::VcdModule,
            "begin" => ScopeKind::VcdBegin,
            "fork" => ScopeKind::VcdFork,
            "function" => ScopeKind::VcdFunction,
            "task" => ScopeKind::VcdTask,
            _ => ScopeKind::Other,
        };
        Scope {
            kind,
            name: name.to_string(),
        }
    }
}

/// Analoguous to VariableInfo (for VCD), the two representation will be merged soon
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FstHeader {
    pub variables: Vec<VariableInfo>,
}
