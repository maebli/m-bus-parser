//! Prints "Hello, world!" on the host console using semihosting

#![no_main]
#![no_std]

use core::panic::PanicInfo;

use cortex_m_semihosting::{debug, hprintln};
use m_bus_parser::{
    frames::Frame,
    user_data::{
        data_record::{self, DataRecord},
        DataRecords,
    },
    MbusData,
};

use cortex_m_rt::entry;

#[panic_handler]
#[no_mangle]
fn my_panic_handler(info: &PanicInfo) -> ! {
    hprintln!("Oh noes, panic {:?} :(", info);
    loop {}
}

#[entry]
fn main() -> ! {
    let data = [0x03, 0x13, 0x15, 0x31, 0x00, 0x03, 0x13, 0x15, 0x31, 0x00];
    let data_records = DataRecords::try_from(data.as_slice());

    hprintln!("{:?}", data_records);

    debug::exit(debug::EXIT_SUCCESS);

    loop {}
}
