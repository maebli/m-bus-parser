#[cfg(feature = "decryption-mode-7")]
use super::OutputError;
use super::{DecodedOutput, OmsReport};
use m_bus_oms_rules::firmware::ImageTransfer;
use serde_json::Value;

pub(super) fn decode(data: &[u8]) -> Value {
    serde_json::json!(ImageTransfer::parse(data))
}

pub(super) fn assess(frame: &DecodedOutput, report: &mut OmsReport) {
    let Some(app) = &frame.application else {
        return;
    };
    let Some(decoded) = &app.decoded else {
        return;
    };
    let wrapped = app.protocol == "security_information_transfer";
    let command = matches!(app.control_information.as_str(), "C0" | "C7" | "C3" | "C6");
    let blocks = decoded["blocks"]
        .as_array()
        .map(Vec::as_slice)
        .unwrap_or_else(|| std::slice::from_ref(decoded));
    for block in blocks {
        let Some(value) = block.get("image_transfer") else {
            continue;
        };
        let firmware = serde_json::from_value::<ImageTransfer>(value.clone()).unwrap_or_default();
        let wrapper = (|| {
            let expected_time = if firmware.function & 0x7f == 6 {
                "0000008030"
            } else {
                "0000000030"
            };
            Some((
                block.get("block_control")?.as_u64()? as u8,
                block.get("key_id")?.as_u64()? as u8,
                block.get("data_selector")?.as_u64()? as u8,
                block.get("target_time_hex")?.as_str()? == expected_time,
            ))
        })();
        firmware.assess(wrapped, wrapper, command, report);
    }
}

#[cfg(feature = "decryption-mode-7")]
impl DecodedOutput {
    pub fn verify_firmware_image(
        &mut self,
        image: &[u8],
        image_key: &[u8; 16],
    ) -> Result<(), OutputError> {
        let Some(decoded) = self
            .application
            .as_mut()
            .and_then(|app| app.decoded.as_mut())
        else {
            return Ok(());
        };
        let verify = |block: &mut Value| {
            let Some(fields) = block.get_mut("image_transfer") else {
                return;
            };
            if fields["function"] != 0
                || fields["mac_algorithm"] != 4
                || fields["mac_key_id"] != 0x18
            {
                return;
            }
            if let Ok(tag) = serde_json::from_value::<[u8; 16]>(fields["mac"].clone()) {
                fields["firmware_cmac_matches"] =
                    m_bus_core::decryption::verify_firmware_cmac(image, &tag, image_key).into();
            }
        };
        if let Some(blocks) = decoded.get_mut("blocks").and_then(Value::as_array_mut) {
            blocks.iter_mut().for_each(verify);
        } else {
            verify(decoded);
        }
        Ok(())
    }
}
