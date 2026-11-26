use super::DecodedOutput;
use crate::output::{decode_hex_bytes, RecordAvailability};
use m_bus_oms_rules::telegram::{ApplicationProtocolEvidence, DlmsAssociation};

pub(in crate::output) fn application_protocol(
    frame: &DecodedOutput,
) -> ApplicationProtocolEvidence {
    let mut evidence = ApplicationProtocolEvidence::default();
    let Some(application) = frame
        .application
        .as_ref()
        .filter(|app| matches!(app.protocol.as_str(), "sml" | "dlms_cosem"))
    else {
        return evidence;
    };
    let clear = frame.security.decrypted_payload_hex.as_deref().or_else(|| {
        (!matches!(
            frame.record_availability,
            RecordAvailability::Encrypted | RecordAvailability::Fragmented
        ))
        .then_some(application.payload_hex.as_str())
    });
    let Some(bytes) = clear.and_then(|hex| decode_hex_bytes(hex).ok()) else {
        return evidence;
    };
    if application.protocol == "dlms_cosem" {
        let bytes = if frame.security.decrypted_payload_hex.is_some()
            && frame.security.mode_code == Some(5)
        {
            bytes.strip_prefix(&[0x2f, 0x2f]).unwrap_or(&bytes)
        } else {
            &bytes
        };
        evidence.association = DlmsAssociation::from_mbus_payload(bytes);
    } else {
        #[cfg(feature = "oms-gen3")]
        {
            crate::output::sml::inspect(&bytes, &mut evidence);
        }
    }
    evidence
}
