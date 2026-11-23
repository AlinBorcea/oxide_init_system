#![no_std]
#![no_main]

#[path ="../data/mod.rs"]
mod data;
use data::unit_file::*;
use data::pager::*;
use pager::*;
use simple_pager::SimplePager;
use multi_pager::MultiPager;
use pager_set::PagerSet;

#[path ="../unit/read.rs"]
mod read;
use read as unit_read;
use read::DirEntry;

#[path = "../runner/mod.rs"]
mod runner;
use runner::run;

use core::panic::PanicInfo;
use core::fmt::Write;
use core::ffi::CStr;

use libc::{c_int, c_void};

#[link(name = "c")]
unsafe extern "C" {}

#[unsafe(no_mangle)]
pub extern "C" fn rust_eh_personality() {}

struct StderrWriter;

impl core::fmt::Write for StderrWriter {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        unsafe {
            libc::write(2, s.as_ptr() as *const core::ffi::c_void, s.len());
        }
        Ok(())
    }
} 

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    let mut writer = StderrWriter;

    // Print the main panic header
    let _ = core::write!(writer, "\n[Daemonizer Panic]: ");
    
    // Print the core message string if available
    if let Some(msg) = info.message().as_str() {
        let _ = writer.write_str(msg);
    } else {
        let _ = core::write!(writer, "{}", info.message());
    }

    // Print exact source file, line, and column coordinates
    if let Some(location) = info.location() {
        let _ = core::write!(
            writer,
            "\n  at {}:{}:{}\n",
            location.file(),
            location.line(),
            location.column()
        );
    } else {
        let _ = writer.write_str("\n");
    }

    unsafe {
        libc::_exit(101);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *const *const u8) -> c_int {
    //let pager = SimplePager::new();
    run_read_loop()
}

fn run_read_loop() -> c_int {
    let path = c".";
    let dir_guard_opt = unit_read::open_dir(path);
    if let None = dir_guard_opt {
        unsafe { libc::write(1, b"Could not open dir\n".as_ptr() as *mut c_void, 19); }
        return 1;
    }

    let mut entry = DirEntry { filename: [0;256], written: 0};
    let mut dir_guard = dir_guard_opt.unwrap();
    let ext = b".txt";

    let pager_opt = SimplePager::new(1, 1024, 512, 20, 5, 1);
    let mut pager = pager_opt.unwrap();

    let multi_pager_opt = MultiPager::new(1, 20, 100, 10, 5, 1);
    let mut multi_pager = multi_pager_opt.unwrap();

    let pager_set_opt = PagerSet::new(1, 1024, 128, 20, 5, 1);
    let mut pager_set = pager_set_opt.unwrap();

    let unit_size = core::mem::size_of::<Option<Unit>>();
    let units = unsafe { libc::malloc(unit_size * 1000) as *mut Option<Unit> };
    let unit_slice = unsafe { core::slice::from_raw_parts_mut(units, 1000) };
    let mut unit_length = 0;

    loop {
        let code = unit_read::read_dir_entry(&mut dir_guard, &mut entry);
        if code != 0 {
            break;
        }

        let cstr = match CStr::from_bytes_until_nul(&entry.filename) {
            Ok(s) => s,
            Err(_) => continue,
        };

        let bytes = cstr.to_bytes();

        // Guard: Filename must be strictly longer than the extension itself
        if bytes.len() <= ext.len() {
            continue;
        }

        if !bytes.ends_with(ext) {
            continue;
        }
        
        let valid_slice = &entry.filename[..entry.written as usize];

        let path = match core::ffi::CStr::from_bytes_with_nul(valid_slice) {
            Ok(s) => s,
            Err(_) => continue,
        };

        match run_parse_unit(path, &mut pager, &mut multi_pager, &mut pager_set) {
            Some(unit) => { 
                unit_slice[unit_length] = Some(unit); 
                unit_length += 1; 
            },
            None => {}
        }
    }

    let mut count_to_run = unit_length;
    loop {
        if count_to_run == 0 {
            break;
        }

        let mut progressed = false;

        // 1. Find a unit with in_degree == 0 that hasn't been processed yet
        for i in 0..unit_length {
            let zero_unit = match &unit_slice[i] {
                Some(u) => u,
                None => continue,
            };

            // If in_degree is not 0, or if it's already marked as processed (u16::MAX), skip
            if zero_unit.unit_section.in_degree != 0 {
                continue;
            }

            let current_identity = match zero_unit.id {
                Some(id) => id,
                None => continue
            };

            let name = pager_set.find(&current_identity);
            let path = pager.find(&zero_unit.service.exec_start_handle);

            // Print the ready unit
            unsafe {
                /*libc::write(1, b"Zero degree unit: ".as_ptr() as *const c_void, 18);
                libc::write(1, name.as_ptr() as *const c_void, name.count_bytes());
                libc::write(1, b" : path= ".as_ptr() as *const c_void, 9);
                libc::write(1, path.as_ptr() as *const c_void, path.count_bytes());
                libc::write(1, b"\n".as_ptr() as *const c_void, 1);*/
                _ = run::run_unit(zero_unit, &path);
            }

            // MARK AS PROCESSED: Set in_degree to u16::MAX so we never pick it up again
            unit_slice[i].as_mut().unwrap().unit_section.in_degree = u16::MAX;

            // 2. Scan all other units by index and decrement their in_degree if they depend on this one
            for j in 0..unit_length {
                if i == j {
                    continue;
                }

                let target_unit = match &mut unit_slice[j] {
                    Some(u) => u,
                    None => continue,
                };

                let after_handle = match target_unit.unit_section.after_handle {
                    Some(h) => h,
                    None => continue,
                };

                let after_handles = multi_pager.find(&after_handle);

                // FIXED: Check if target depends on current_identity (the zero-degree unit)
                if depends_on(after_handles, &current_identity) {
                    if target_unit.unit_section.in_degree > 0 && target_unit.unit_section.in_degree != u16::MAX {
                        target_unit.unit_section.in_degree -= 1;
                        /*unsafe {
                            let id = match target_unit.id {
                                Some(id) => id,
                                None => continue
                            };

                            let name = pager_set.find(&id);

                            libc::write(1, name.as_ptr() as *const c_void, name.count_bytes());
                            libc::write(1, b" depends\n".as_ptr() as *const c_void, 9);
                        }*/
                    }
                }
            }

            count_to_run -= 1;
            progressed = true;
            break; // Break the inner search loop to restart scanning from the top with updated in_degrees
        }

        // If we looped through everything and found no 0-degree units left, but count_to_run > 0, it's a deadlock!
        if !progressed {
            unsafe { libc::write(1, b"Circular dependency detected!\n".as_ptr() as *const c_void, 30); }
            break;
        }
    }

    unsafe { libc::free(units as *mut c_void); }
    0
}

fn run_parse_unit(path: &CStr, pager: &mut SimplePager, multi_pager: &mut MultiPager, pager_set: &mut PagerSet) -> Option<Unit> {
    let fd = unsafe { libc::open(path.as_ptr(), libc::O_RDONLY) };
    if fd < 0 {
        return None;
    }

    let mut buffer = [0u8;512];
    let unit = unit_read::read_unit(path, fd, &mut buffer, pager, multi_pager, pager_set);
    unit
}

fn depends_on(after_handles: &[Handle], identity: &Handle) -> bool {
    for handle in after_handles {
        if *handle == *identity {
            return true;
        }
    }
    false
}