#![no_std]
#![no_main]

use core::panic::PanicInfo;

use libc::{c_int};

#[link(name = "c")]
unsafe extern "C" {}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    unsafe { libc::_exit(101) };
}

#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *const *const u8) {
    //let pager = SimplePager::new();
}