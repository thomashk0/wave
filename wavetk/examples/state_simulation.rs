use wavetk::simulation::StateSimulation;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1).expect("Need 1 argument");
    let mut s = StateSimulation::new(&input).unwrap();
    s.load_header()?;
    s.allocate_state()?;
    let mut w: i64 = 0;
    let mut i = 0;
    while !s.done() {
        let r = s.next_cycle()?;
        w += r.0;
        let total: i8 = r.1.iter().sum();
        w += total as i64;
        i += 1;
    }
    println!("i = {}", i);
    println!("w = {}", w);
    Ok(())
}
