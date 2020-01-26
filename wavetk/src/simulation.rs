use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::io;

use crate::types::{VariableInfo, VariableKind};
use crate::vcd::{VcdCommand, VcdError, VcdParser, VcdValue};

fn logic_level(c: char) -> i8 {
    match c as u8 {
        b'0' => 0,
        b'1' => 1,
        b'U' | b'u' => -1,
        b'W' | b'w' => -2,
        b'Z' | b'z' => -3,
        b'X' | b'x' => -4,
        _ => -5,
    }
}

/// The StateSimulation recreates the complete state of a circuit over the time
pub struct StateSimulation {
    parser: VcdParser<File>,
    state: Vec<i8>,
    var_offset: HashMap<String, usize>,
    var_width: HashMap<String, usize>,
    tracked_var: HashSet<String>,
    previous_cycle: i64,
    current_cycle: i64,
}

impl StateSimulation {
    pub fn new(filename: &str) -> io::Result<Self> {
        const N_VAR: usize = 2048;
        let f = File::open(filename)?;
        Ok(StateSimulation {
            parser: VcdParser::with_chunk_size(4096, f),
            state: Vec::with_capacity(N_VAR),
            var_offset: HashMap::with_capacity(N_VAR),
            var_width: HashMap::with_capacity(N_VAR),
            tracked_var: HashSet::new(),
            previous_cycle: -1,
            current_cycle: -1,
        })
    }

    pub fn state(&self) -> &[i8] {
        &self.state
    }

    pub fn track_variables(&mut self, vars: &[&str]) {
        self.tracked_var.extend(vars.iter().map(|s| s.to_string()));
    }

    pub fn allocate_state(&mut self) -> Result<(), VcdError> {
        let mut offset = 0usize;
        let variables = &self
            .parser
            .header()
            .ok_or(VcdError::PartialHeader)?
            .variables;

        self.var_offset.clear();
        self.var_width.clear();
        for v in variables {
            if self.var_offset.get(&v.id).is_some() {
                // It seems legal that several variables map to the same ID. For example the
                // clock is defined in many component but they all map to the same ID.
                //
                // FIXME: maybe the header should be checked for correctness upon load?
                assert_eq!(self.var_width.get(&v.id).cloned(), Some(v.width as usize));
                continue;
            }
            if v.kind == VariableKind::VcdReal {
                continue;
            }
            if !self.tracked_var.is_empty() && !self.tracked_var.contains(&v.id) {
                continue;
            }
            self.var_offset.insert(v.id.clone(), offset);
            self.var_width.insert(v.id.clone(), v.width as usize);
            offset += v.width as usize;
        }
        self.state.resize(offset, 0);
        Ok(())
    }

    pub fn header_info(&self) -> Result<HashMap<&str, (Option<usize>, VariableInfo)>, VcdError> {
        let variables = &self
            .parser
            .header()
            .ok_or(VcdError::PartialHeader)?
            .variables;
        let mut w: HashMap<&str, (Option<usize>, VariableInfo)> =
            HashMap::with_capacity(variables.len());
        for v in variables {
            w.insert(&v.id, (self.var_offset.get(&v.id).cloned(), v.clone()));
        }
        Ok(w)
    }

    pub fn load_header(&mut self) -> Result<(), VcdError> {
        self.parser.load_header()?;
        Ok(())
    }

    pub fn done(&self) -> bool {
        self.parser.done()
    }

    pub fn next_cycle(&mut self) -> Result<(i64, &[i8]), VcdError> {
        let state = &mut self.state;
        let var_offset = &self.var_offset;
        let var_width = &self.var_width;
        let tracked_var = &self.tracked_var;
        let mut cycle = 0;
        let callback = |cmd: VcdCommand| {
            match cmd {
                VcdCommand::SetCycle(c) => {
                    cycle = c as i64;
                    return true;
                }
                VcdCommand::ValueChange(v) => {
                    if !tracked_var.is_empty() && !tracked_var.contains(v.var_id) {
                        return false;
                    }
                    let base = var_offset
                        .get(v.var_id)
                        .cloned()
                        .expect(&format!("missing key {}", v.var_id));
                    match v.value {
                        VcdValue::Bit(c) => state[base] = logic_level(c),
                        VcdValue::Vector(x) => {
                            let w = var_width.get(v.var_id).cloned().unwrap();
                            assert_eq!(w, x.len());
                            for (el, c) in state[base..base + w].iter_mut().zip(x.chars()) {
                                *el = logic_level(c);
                            }
                        }
                        VcdValue::Real(_) => {}
                    };
                }
                VcdCommand::Directive(_) | VcdCommand::VcdEnd => {}
            }
            false
        };
        self.parser.process_vcd_commands(callback)?;

        self.previous_cycle = self.current_cycle;
        self.current_cycle = cycle;
        Ok((self.previous_cycle, &self.state))
    }
}
