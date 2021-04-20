use std::path::PathBuf;
use wavetk::simulation::StateSimulation;

fn vcd_asset(rel_path: &str) -> PathBuf {
    let mut path = PathBuf::from(file!());
    path.pop();
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
    sim.allocate_state()?;
    let clk_id = sim.header_info()?.get("!").unwrap().0.unwrap();

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


#[test]
fn sim_picorv32() -> Result<(), Box<dyn std::error::Error>> {
    // NOTE: this test is constructed from a file that failed initialy.
    // Random constants were obtained by manual inspection of the .vcd.

    let f = vcd_asset("good/picorv32_iverilog.vcd");
    let mut sim = StateSimulation::new(f.to_str().unwrap())?;
    sim.load_header()?;
    sim.allocate_state()?;

    let sig = sim.header_info()?.get("a\"").unwrap().0.unwrap();
    let sig_w = 128usize;

    let (c, d) = sim.next_cycle()?;
    assert_eq!(c, -1);
    assert!(d[sig..sig + sig_w].iter().all(|x| *x == 0));

    // Check left extension worked
    let (c, d) = sim.next_cycle()?;
    assert_eq!(c, 0);
    assert!(d[sig..sig + 91].iter().all(|x| *x == 1));

    let (c, _) = sim.next_cycle()?;
    assert_eq!(c, 5000);
    Ok(())
}