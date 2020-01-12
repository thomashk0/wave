use wavetk::fst::FstReader;

pub fn main() -> Result<(), Box<dyn std::error::Error>> {
    let input = std::env::args().nth(1).expect("Need 1 argument");
    let mut reader = FstReader::from_file(&input, false).expect("unable to open reader");
    println!("FstReader@{:?}", reader);
    println!("    file_type      = {:?}", reader.file_type());
    println!("    version_string = {:?}", reader.version_string());
    println!("    date_string    = {:?}", reader.date_string());
    println!("    var_count      = {}", reader.var_count());
    println!("    max_handle     = {}", reader.max_handle());
    println!("    time_zero      = {}", reader.time_zero());
    println!("    start_time     = {}", reader.start_time());
    println!("    end_time       = {}", reader.end_time());

    let header = reader.load_header();
    for v in &header.variables {
        println!("{:?}", v);
    }

    //    reader.iter_hier(dump_fst_hier);
    //    reader.iter_blocks(|cycle, var_handle, value| {
    //        let value_str = unsafe { CStr::from_ptr(value as *const c_char).to_str().unwrap() };
    //        println!("{:4} {:3} -> {}", cycle, var_handle, value_str);
    //    });
    Ok(())
}
