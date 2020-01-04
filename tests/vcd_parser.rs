use std::fs::File;
use std::path::PathBuf;

use wave::vcd::{VcdHeader, VcdParser};

fn vcd_asset(rel_path: &str) -> PathBuf {
    let mut path = PathBuf::from(file!());
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

#[test]
fn parse_ghdl_0() -> Result<(), Box<dyn std::error::Error>> {
    let (header, n_cmd) = parse_file(&vcd_asset("good/ghdl_0.vcd"), 3)?;
    assert_eq!(header.variables.len(), 10);
    assert_eq!(n_cmd, 29);
    Ok(())
}

#[test]
fn parse_header_0() -> Result<(), Box<dyn std::error::Error>> {
    let (header, n_cmd) = parse_file(&vcd_asset("good/header_0.vcd"), 128)?;
    assert_eq!(header.variables.len(), 3);
    assert_eq!(n_cmd, 3);
    Ok(())
}
