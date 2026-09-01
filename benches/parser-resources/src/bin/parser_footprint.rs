#![cfg_attr(target_os = "none", no_std)]
#![cfg_attr(target_os = "none", no_main)]

use m_bus_parser_resources::{parse_full_wired_frame, FULL_FRAME};

#[cfg(target_os = "none")]
use core::hint::black_box;
#[cfg(not(target_os = "none"))]
use std::hint::black_box;

fn parse_frame() {
    let result = parse_full_wired_frame(black_box(&FULL_FRAME));
    let _ = black_box(result);
}

#[cfg(not(target_os = "none"))]
fn main() {
    parse_frame();
}

#[cfg(target_os = "none")]
#[no_mangle]
pub extern "C" fn _start() -> ! {
    parse_frame();
    loop {
        core::hint::spin_loop();
    }
}

#[cfg(target_os = "none")]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
