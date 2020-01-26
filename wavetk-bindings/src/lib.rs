use std::ffi::{CStr, CString};
use std::os::raw::c_char;
use std::ptr::null_mut;

use std::num::ParseIntError;
use std::slice;
use wavetk::simulation::StateSimulation;
use wavetk::vcd::VcdError;

const VERSION_MAJOR: &'static str = env!("CARGO_PKG_VERSION_MAJOR");
const VERSION_MINOR: &'static str = env!("CARGO_PKG_VERSION_MINOR");
const VERSION_PATCH: &'static str = env!("CARGO_PKG_VERSION_PATCH");

/// FFI error codes, encoded as an i32
type WaveTkStatus = i32;

fn encode_error(err: VcdError) -> WaveTkStatus {
    match err {
        VcdError::IoError(_) => 1,
        VcdError::ParseError => 2,
        VcdError::MissingData => 3,
        VcdError::PartialHeader => 4,
        VcdError::Utf8Error => 5,
        VcdError::EndOfInput => 6,
    }
}

/// Get the (major, minor, patch) triple for this crate version
fn get_version() -> Result<(u8, u8, u8), ParseIntError> {
    let major = VERSION_MAJOR.parse::<u8>()?;
    let minor = VERSION_MINOR.parse::<u8>()?;
    let patch = VERSION_PATCH.parse::<u8>()?;
    Ok((major, minor, patch))
}

#[no_mangle]
pub extern "C" fn wavetk_version() -> u32 {
    let v = get_version().unwrap_or((0, 0, 0));
    (v.0 as u32) << 16 | (v.1 as u32) << 8 | (v.2 as u32)
}

#[no_mangle]
pub unsafe extern "C" fn wave_sim_create(
    filename: *const c_char,
    status: *mut i32,
) -> *mut StateSimulation {
    assert!(!filename.is_null());
    let f_name = CStr::from_ptr(filename).to_str();
    if f_name.is_err() {
        *status = encode_error(VcdError::Utf8Error);
        return null_mut();
    }
    match StateSimulation::new(f_name.unwrap()) {
        Ok(sim) => Box::into_raw(Box::new(sim)),
        Err(e) => {
            *status = encode_error(VcdError::IoError(e));
            null_mut()
        }
    }
}

#[no_mangle]
pub extern "C" fn wave_sim_load_header(ptr: *mut StateSimulation) -> WaveTkStatus {
    assert!(!ptr.is_null());
    let sim = unsafe { &mut *ptr };
    match sim.load_header() {
        Ok(_) => 0,
        Err(e) => encode_error(e),
    }
}

#[no_mangle]
pub extern "C" fn wave_sim_allocate_state(
    ptr: *mut StateSimulation,
    restrict: *const *const c_char,
    n: usize,
) -> WaveTkStatus {
    assert!(!ptr.is_null());
    let sim = unsafe { &mut *ptr };
    if !restrict.is_null() && n > 0 {
        let names_ptr = unsafe { slice::from_raw_parts(restrict, n as usize) };
        let mut vars: Vec<&str> = Vec::with_capacity(n);
        for name_ptr in names_ptr {
            let name = unsafe { CStr::from_ptr(*name_ptr).to_str() };
            if name.is_err() {
                return encode_error(VcdError::Utf8Error);
            }
            vars.push(name.unwrap());
        }
        sim.track_variables(&vars);
    }

    match sim.allocate_state() {
        Ok(_) => 0,
        Err(e) => encode_error(e),
    }
}

#[no_mangle]
pub unsafe extern "C" fn wave_sim_header_info(ptr: *const StateSimulation) -> *mut c_char {
    assert!(!ptr.is_null());
    let sim = &*ptr;
    let header = sim.header_info();
    if header.is_err() {
        return null_mut();
    }
    let header_str = serde_json::to_string(&header.unwrap());
    match header_str {
        Ok(s) => {
            let c_str = CString::new(s).unwrap();
            c_str.into_raw()
        }
        Err(_) => null_mut(),
    }
}

/// Retrieve the internal state buffer pointer an size.
///
/// Important: it gets invalidated by calls to allocate_state.
#[no_mangle]
pub unsafe extern "C" fn wavetk_sim_state_buffer(
    ptr: *mut StateSimulation,
    data: *mut *const i8,
    size: *mut u64,
) -> WaveTkStatus {
    assert!(!ptr.is_null());
    let sim = &mut *ptr;
    *data = sim.state().as_ptr();
    *size = sim.state().len() as u64;
    0
}

#[no_mangle]
pub unsafe extern "C" fn wave_sim_next_cycle(
    ptr: *mut StateSimulation,
    cycle: *mut i64,
    data: *mut *const i8,
    size: *mut u64,
) -> WaveTkStatus {
    assert!(!ptr.is_null());
    let sim = &mut *ptr;
    if sim.done() {
        return encode_error(VcdError::EndOfInput);
    }
    match sim.next_cycle() {
        Ok((c, state)) => {
            *cycle = c;
            *data = state.as_ptr();
            *size = state.len() as u64;
            0
        }
        Err(e) => encode_error(e),
    }
}

#[no_mangle]
pub extern "C" fn wave_sim_destroy(p: *mut StateSimulation) {
    if p.is_null() {
        return;
    }
    unsafe {
        Box::from_raw(p);
    }
}

#[no_mangle]
pub extern "C" fn wave_str_destroy(p: *const c_char) {
    if p.is_null() {
        return;
    }
    unsafe {
        CStr::from_ptr(p);
    }
}
