#![no_std]
#![no_main]

use cortex_m_semihosting::{debug, hprintln};
use m_bus_parser::{frames::Frame, user_data::data_record::DataRecord, MbusData};
// pick a panicking behavior
use panic_halt as _; // you can put a breakpoint on `rust_begin_unwind` to catch panics
                     // use panic_abort as _; // requires nightly
                     // use panic_itm as _; // logs messages over ITM; requires ITM support
                     // use panic_semihosting as _; // logs messages to the host stderr; requires a debugger

use cortex_m::asm;
use cortex_m_rt::entry;

#[entry]
fn main() -> ! {
    let data = [0x03, 0x13, 0x15, 0x31, 0x00];
    let result = DataRecord::try_from(data.as_slice());
    hprintln!("{:?}", result);

    let example = [
        0x68, 0x06, 0x06, 0x68, 0x53, 0xFE, 0x51, 0x01, 0x7A, 0x08, 0x25, 0x16,
    ];

    let frame = Frame::try_from(example.as_slice()).unwrap();

    hprintln!("{:?}", frame);
    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}
