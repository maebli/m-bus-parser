#![no_std]
#[path = "../../full_decode.rs"]
mod full_decode;

/// # Safety
/// The caller must provide a readable slice of `length` bytes.
#[no_mangle]
pub unsafe extern "C" fn comparison_decode(
    bytes: *const u8,
    length: usize,
) -> full_decode::Decoded {
    full_decode::decode(core::slice::from_raw_parts(bytes, length))
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
