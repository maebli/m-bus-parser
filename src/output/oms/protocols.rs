use super::DecodedOutput;
use crate::output::RecordAvailability;
use m_bus_oms_rules::telegram::ApplicationProtocolEvidence;

pub(in crate::output) fn application_protocol(
    frame: &DecodedOutput,
) -> ApplicationProtocolEvidence {
    let mut evidence = ApplicationProtocolEvidence::default();
    let ci = frame.transport.application_ci.or(frame.transport.outer_ci);
    if let Some(ci @ (0x50 | 0x53..=0x57 | 0x66..=0x68)) = ci {
        if frame.security.decryption_state == "decrypted_authenticated" {
            evidence.selection_length = frame
                .security
                .decrypted_payload_hex
                .as_ref()
                .map(|payload| payload.len() / 2);
            return evidence;
        }
        if matches!(
            frame.record_availability,
            RecordAvailability::Encrypted | RecordAvailability::Fragmented
        ) || frame.security.mode_code.is_some_and(|mode| mode != 0)
        {
            return evidence;
        }
        evidence.selection_length = frame
            .application
            .as_ref()
            .filter(|app| {
                matches!(
                    app.protocol.as_str(),
                    "request_selected_application"
                        | "selected_application"
                        | "application_reset_or_select"
                )
            })
            .map(|app| app.payload_hex.len() / 2)
            .or_else(|| {
                let wire = frame.raw.payload_hex.as_deref()?;
                let header = match ci {
                    0x50 | 0x54 | 0x66 => 0,
                    0x56 | 0x57 | 0x67 => 4,
                    _ => 12,
                };
                (frame.transport.outer_ci == Some(ci))
                    .then(|| (wire.len() / 2).checked_sub(1 + header))?
            });
        return evidence;
    }
    evidence
}
