//! Link and run a custom decoder without an allocator or std.
#![no_main]
#![no_std]

use cortex_m_rt::entry;
use cortex_m_semihosting::debug;
use m_bus_manufacturer::{
    Cursor, DecodeError, DecoderDescriptor, Field, ManufacturerDecoder, MeterInfo, Registry,
    Selector,
};
use panic_halt as _;

struct Example;
impl ManufacturerDecoder for Example {
    fn descriptor(&self) -> &'static DecoderDescriptor {
        static DESCRIPTOR: DecoderDescriptor = DecoderDescriptor {
            name: "Synthetic embedded example",
            source: "Synthetic example, not a vendor decoder",
            selector: Selector {
                manufacturer: *b"ABC",
                versions: None,
                device: None,
            },
        };
        &DESCRIPTOR
    }
    fn decode(
        &self,
        _: &MeterInfo,
        tail: &[u8],
        emit: &mut dyn FnMut(Field<'_>),
    ) -> Result<usize, DecodeError> {
        let mut cursor = Cursor::new(tail);
        emit(Field::unsigned("counter", cursor.u16_le()?, 0..2));
        Ok(cursor.position())
    }
}

#[entry]
fn main() -> ! {
    let meter = MeterInfo {
        manufacturer: Some(*b"ABC"),
        ..MeterInfo::default()
    };
    let mut fields = 0;
    let result = Registry::only(&[&Example]).decode(&meter, &[0x34, 0x12], &mut |_| fields += 1);
    let success = matches!(result, Some(Ok(_))) && fields == 1;
    debug::exit(if success {
        debug::EXIT_SUCCESS
    } else {
        debug::EXIT_FAILURE
    });
    loop {
        cortex_m::asm::nop();
    }
}
