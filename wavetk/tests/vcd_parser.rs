use std::fs::File;
use std::path::PathBuf;

use wavetk::vcd::{VcdHeader, VcdParser};

fn vcd_asset(rel_path: &str) -> PathBuf {
    let mut path = PathBuf::from(file!());
    path.pop();
    path.pop();
    path.pop();
    path.push("assets/vcd");
    path.push(rel_path);
    path
}

fn parse_file(
    filepath: &PathBuf,
    chunk_size: usize,
) -> Result<(VcdHeader, usize), Box<dyn std::error::Error>> {
    let f = File::open(filepath)?;
    let mut parser = VcdParser::with_chunk_size(chunk_size, f);
    let header = parser.load_header()?.clone();
    let mut cnt = 0;
    parser.process_vcd_commands(|_cmd| {
        cnt += 1;
        false
    })?;
    Ok((header, cnt))
}

fn check_file(
    path: &str,
    chunk_size: usize,
    n_var: usize,
    n_cmd: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    let (header, cmd_count) = parse_file(&vcd_asset(path), chunk_size)?;
    assert_eq!(header.variables.len(), n_var);
    assert_eq!(cmd_count, n_cmd);
    Ok(())
}

macro_rules! parse_tests {
    ($(($name:ident, $path:expr, $chunk_size:expr, $n_var:expr, $n_cmd:expr),)*) => {
    $(
        #[test]
        fn $name() -> Result<(), Box<dyn std::error::Error>> {
            check_file($path, $chunk_size, $n_var, $n_cmd)
        }
    )*
    }
}

parse_tests! {
    (parse_ghdl_3, "good/ghdl_0.vcd", 3, 10, 29),
    (parse_ghdl_13, "good/ghdl_0.vcd", 13, 10, 29),
    (parse_ghdl_128, "good/ghdl_0.vcd", 128, 10, 29),
    (parse_simple_10, "good/simple_0.vcd", 10, 1, 18),
    (parse_simple_1024, "good/simple_0.vcd", 1024, 1, 18),
    (parse_synopsys_256, "good/synopsys_vcd_0.vcd", 256, 16, 55),
    (parse_ieee_1364_2001_16, "good/ieee_1364_2001_sample.vcd", 16, 5, 50),
    (parse_ncsim_32, "good/ncsim_0.vcd", 32, 3, 55),
    (parse_ncsim_4096, "good/ncsim_0.vcd", 4096, 3, 55),
    (parse_verilator_31, "good/verilator_riscv.vcd", 31, 2102, 7230),
    (parse_verilator_4096, "good/verilator_riscv.vcd", 4096, 2102, 7230),
    (parse_iverilog_4096, "good/picorv32_iverilog.vcd", 4096, 418, 355),
}

#[test]
fn parse_header_0() -> Result<(), Box<dyn std::error::Error>> {
    let (header, n_cmd) = parse_file(&vcd_asset("good/header_0.vcd"), 128)?;
    assert_eq!(header.variables.len(), 3);
    assert_eq!(n_cmd, 3);
    Ok(())
}
