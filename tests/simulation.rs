use std::path::PathBuf;
use wave::simulation::StateSimulation;

fn vcd_asset(rel_path: &str) -> PathBuf {
    let mut path = PathBuf::from(file!());
    path.pop();
    path.pop();
    path.push("assets/vcd");
    path.push(rel_path);
    path
}

#[test]
fn sim_ghdl_0() -> Result<(), Box<dyn std::error::Error>> {
    let f = vcd_asset("good/ghdl_0.vcd");
    let mut sim = StateSimulation::new(f.to_str().unwrap())?;
    sim.load_header()?;

    let clk_id = sim.header_info()?.get("!").unwrap().0;
    let (c, d) = sim.next_cycle()?;
    assert_eq!(c, -1);
    assert_eq!(d.len(), 289);
    assert_eq!(d[clk_id], 0);

    let (c, d) = sim.next_cycle()?;
    assert_eq!(c, 0);
    assert_eq!(d[clk_id], 0);

    let (c, d) = sim.next_cycle()?;
    assert_eq!(c, 5000000);
    assert_eq!(d[clk_id], 1);
    Ok(())
}
