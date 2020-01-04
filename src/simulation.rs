use std::collections::HashMap;
use std::fs::File;
use std::io;

use crate::vcd::{VcdCommand, VcdError, VcdParser, VcdValue, VcdVariable};

fn logic_level(c: char) -> i8 {
    match c {
        '0' => 0,
        '1' => 1,
        'U' | 'u' => -1,
        'W' | 'w' => -2,
        'Z' | 'z' => -3,
        'X' | 'x' => -4,
        _ => -5,
    }
}

/// The StateSimulation recreates the complete state of a circuit over the time
pub struct StateSimulation {
    parser: VcdParser<File>,
    state: Vec<i8>,
    var_map: HashMap<String, usize>,
    var_width: HashMap<String, usize>,
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
            var_map: HashMap::with_capacity(N_VAR),
            var_width: HashMap::with_capacity(N_VAR),
            previous_cycle: -1,
            current_cycle: -1,
        })
    }

    fn alloc_variables(&mut self) -> Result<(), VcdError> {
        let mut offset = 0usize;
        let variables = &self
            .parser
            .header()
            .ok_or(VcdError::PartialHeader)?
            .variables;
        for v in variables {
            if v.vtype == "real" {
                continue;
            }
            self.var_map.insert(v.id.clone(), offset);
            self.var_width.insert(v.id.clone(), v.width as usize);
            offset += v.width as usize;
        }
        self.state.resize(offset, 0);
        Ok(())
    }

    pub fn header_info(&self) -> Result<HashMap<&str, (usize, VcdVariable)>, VcdError> {
        let var_info = &self
            .parser
            .header()
            .ok_or(VcdError::PartialHeader)?
            .variables;
        let mut w: HashMap<&str, (usize, VcdVariable)> = HashMap::with_capacity(var_info.len());
        for v in var_info {
            w.insert(&v.id, (*self.var_map.get(&v.id).unwrap(), v.clone()));
        }
        Ok(w)
    }

    pub fn load_header(&mut self) -> Result<(), VcdError> {
        self.parser.load_header()?;
        self.alloc_variables()
    }

    pub fn done(&self) -> bool {
        self.parser.done()
    }

    pub fn next_cycle(&mut self) -> Result<(i64, &[i8]), VcdError> {
        let state = &mut self.state;
        let var_map = &self.var_map;
        let var_width = &self.var_width;
        let mut cycle = 0;
        let callback = |cmd: VcdCommand| {
            match cmd {
                VcdCommand::Directive(_) => {}
                VcdCommand::VcdEnd => {}
                VcdCommand::SetCycle(c) => {
                    cycle = c as i64;
                    return true;
                }
                VcdCommand::ValueChange(v) => {
                    let base = var_map
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
            }
            false
        };
        self.parser.process_vcd_commands(callback)?;

        self.previous_cycle = self.current_cycle;
        self.current_cycle = cycle;
        Ok((self.previous_cycle, &self.state))
    }
}
