#![no_main]

use libfuzzer_sys::fuzz_target;
use wireless_mbus_link_layer::{crc16_en13757, strip_format_a_crcs, FormatAFrame, WirelessFrame};

fuzz_target!(|data: &[u8]| {
    if let Ok(frame) = WirelessFrame::try_from(data) {
        assert_eq!(usize::from(data[0]) + 1, data.len());
        assert!(data[10..].starts_with(frame.data));
    }

    if !(10..=256).contains(&data.len()) {
        return;
    }
    let mut normalized = data.to_vec();
    normalized[0] = (data.len() - 1) as u8;
    let (header, payload) = normalized.split_at(10);
    let mut encoded = Vec::new();
    for block in std::iter::once(header).chain(payload.chunks(16)) {
        encoded.extend_from_slice(block);
        encoded.extend(crc16_en13757(block).to_be_bytes());
    }

    let view = FormatAFrame::new(&encoded).expect("valid Format A header");
    assert_eq!(view.len(), normalized.len());
    assert!(view.bytes().eq(normalized.iter().copied()));
    let mut output = vec![0; encoded.len()];
    assert_eq!(
        strip_format_a_crcs(&encoded, &mut output),
        Some(normalized.as_slice())
    );
});
