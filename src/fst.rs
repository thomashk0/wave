use fst_sys;
use std::ffi::{CStr, CString};
use std::os::raw::{c_char, c_uchar, c_void};
use std::ptr::null_mut;
use std::slice;
use std::str;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FstError {
    InvalidFile,
    InvalidConversion,
    NullPointer,
    Utf8Error,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum FstFileType {
    Verilog,
    Vhdl,
    VerilogVhdl,
}

#[derive(Debug)]
pub struct FstReader {
    handle: *mut c_void,
}

type FstChangeCallback = extern "C" fn(*mut c_void, u64, fst_sys::fstHandle, *const c_uchar);

impl FstReader {
    pub fn from_file(name: &str, use_extensions: bool) -> Result<FstReader, FstError> {
        let p = unsafe { fst_sys::fstReaderOpen(CString::new(name).unwrap().as_ptr()) };
        if p.is_null() {
            return Err(FstError::InvalidFile);
        }
        if use_extensions {
            unsafe {
                fst_sys::fstReaderSetVcdExtensions(p, 1);
            }
        }
        Ok(FstReader { handle: p })
    }

    pub fn iter_hier<F>(&mut self, mut callback: F)
    where
        F: FnMut(&fst_sys::fstHier),
    {
        unsafe {
            fst_sys::fstReaderIterateHierRewind(self.handle);
        }
        loop {
            let p = unsafe {
                let ptr = fst_sys::fstReaderIterateHier(self.handle);
                if ptr.is_null() {
                    None
                } else {
                    Some(&*ptr)
                }
            };
            if p.is_none() {
                break;
            }
            callback(p.unwrap());
        }
    }

    pub fn iter_blocks<F>(&mut self, mut f: F) -> i32
    where
        F: FnMut(u64, fst_sys::fstHandle, *const c_uchar),
    {
        unsafe {
            fst_sys::fstReaderSetFacProcessMaskAll(self.handle);
            let (data, f) = unpack_closure(&mut f);
            fst_sys::fstReaderIterBlocks(self.handle, Some(f), data, null_mut())
        }
    }

    pub fn end_time(&self) -> u64 {
        unsafe { fst_sys::fstReaderGetEndTime(self.handle) }
    }

    pub fn file_type(&self) -> Result<FstFileType, FstError> {
        let w = unsafe { fst_sys::fstReaderGetFileType(self.handle) } as u32;
        match w {
            fst_sys::fstFileType_FST_FT_VERILOG => Ok(FstFileType::Verilog),
            fst_sys::fstFileType_FST_FT_VHDL => Ok(FstFileType::Vhdl),
            fst_sys::fstFileType_FST_FT_VERILOG_VHDL => Ok(FstFileType::VerilogVhdl),
            _ => Err(FstError::InvalidConversion),
        }
    }

    pub fn max_handle(&self) -> u32 {
        unsafe { fst_sys::fstReaderGetMaxHandle(self.handle) }
    }

    pub fn scope_count(&self) -> usize {
        let r = unsafe { fst_sys::fstReaderGetScopeCount(self.handle) };
        r as usize
    }

    pub fn start_time(&self) -> u64 {
        unsafe { fst_sys::fstReaderGetStartTime(self.handle) }
    }

    // The exponent of the timescale, time = cycle 10^(timescale)
    pub fn timescale(&self) -> i8 {
        unsafe { fst_sys::fstReaderGetTimescale(self.handle) }
    }

    pub fn time_zero(&self) -> i64 {
        unsafe { fst_sys::fstReaderGetTimezero(self.handle) }
    }

    pub fn var_count(&self) -> u64 {
        unsafe { fst_sys::fstReaderGetVarCount(self.handle) }
    }

    pub fn version_string(&self) -> Result<&str, FstError> {
        let c_str = unsafe {
            let p = fst_sys::fstReaderGetVersionString(self.handle);
            CStr::from_ptr(p).to_str()
        };
        c_str.or(Err(FstError::Utf8Error))
    }

    pub fn date_string(&self) -> Result<&str, FstError> {
        let c_str = unsafe {
            let p = fst_sys::fstReaderGetDateString(self.handle);
            CStr::from_ptr(p).to_str()
        };
        c_str.or(Err(FstError::Utf8Error))
    }

    pub fn time_range(&mut self, range: Option<(u64, u64)>) {
        match range {
            None => unsafe { fst_sys::fstReaderSetUnlimitedTimeRange(self.handle) },
            Some((start, end)) => unsafe {
                fst_sys::fstReaderSetLimitTimeRange(self.handle, start, end)
            },
        }
    }
}

impl Drop for FstReader {
    fn drop(&mut self) {
        if self.handle.is_null() {
            return;
        }
        unsafe {
            fst_sys::fstReaderClose(self.handle);
        }
    }
}

unsafe fn unpack_closure<F>(closure: &mut F) -> (*mut c_void, FstChangeCallback)
where
    F: FnMut(u64, fst_sys::fstHandle, *const c_uchar),
{
    extern "C" fn trampoline<F>(
        data: *mut c_void,
        time: u64,
        handle: fst_sys::fstHandle,
        value: *const c_uchar,
    ) where
        F: FnMut(u64, fst_sys::fstHandle, *const c_uchar),
    {
        let closure: &mut F = unsafe { &mut *(data as *mut F) };
        (*closure)(time, handle, value);
    }
    (closure as *mut F as *mut c_void, trampoline::<F>)
}

pub fn dump_fst_hier(h: &fst_sys::fstHier) {
    print!("Type: ");
    let from_ptr = |p: *const c_char, v: usize| {
        assert!(!p.is_null());
        unsafe {
            let s = slice::from_raw_parts(p as *const c_uchar, v);
            str::from_utf8(s).unwrap()
        }
    };

    match h.htyp as u32 {
        fst_sys::fstHierType_FST_HT_SCOPE => {
            println!("Scope");
            let x = unsafe { h.u.scope };
            println!("\tname: {}", from_ptr(x.name, x.name_length as usize));
            println!(
                "\tcomponent: {}",
                from_ptr(x.component, x.component_length as usize)
            );
        }
        fst_sys::fstHierType_FST_HT_UPSCOPE => {
            println!("Upscope");
        }
        fst_sys::fstHierType_FST_HT_VAR => {
            println!("Var");
            let x = unsafe { h.u.var };
            println!("\thandle: {}", x.handle);
            println!("\tname: {}", from_ptr(x.name, x.name_length as usize));
            println!("\ttype: {}", x.typ);
            println!("\tlength: {}", x.length);
            println!("\tdirection: {}", x.direction);
        }
        fst_sys::fstHierType_FST_HT_ATTRBEGIN => {
            println!("AttrBegin");
            let x = unsafe { h.u.attr };
            println!(
                "\ttype: {}",
                match x.typ {
                    0 => "MISC",
                    1 => "Array",
                    2 => "Enum",
                    3 => "Pack",
                    _ => "??",
                }
            );
            println!("\tsubtype: {}", x.subtype);
            println!("\targ: {}", x.arg);
            println!("\tname: {:?}", from_ptr(x.name, x.name_length as usize));
        }
        fst_sys::fstHierType_FST_HT_ATTREND => {
            println!("AttrEnd");
        }
        _ => println!("UNKNOWN"),
    }
}
