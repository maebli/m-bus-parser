//! Prints "Hello, world!" on the host console using semihosting

#![no_main]
#![no_std]

use core::panic::PanicInfo;

use cortex_m_semihosting::{debug, hprintln};
use m_bus_parser::{frames::Frame, user_data::data_record::DataRecord, MbusData};

use cortex_m_rt::entry;

#[panic_handler]
#[no_mangle]
fn my_panic_handler(info: &PanicInfo) -> ! {
    hprintln!("Oh noes, panic {:?} :(", info);
    loop {}
}

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
