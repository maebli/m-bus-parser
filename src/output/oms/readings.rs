use super::{ConfigurationField, DecodedOutput, OmsReport};
use m_bus_oms_rules::readings::history::{DeviceReadingHistory, RegisterValue};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct CaptureReadings<'a> {
    meters: BTreeMap<(&'a str, &'a str, u8, u8), DeviceReadingHistory>,
}

impl<'a> CaptureReadings<'a> {
    pub(super) fn observe(
        &mut self,
        frame: &'a DecodedOutput,
        report: &mut OmsReport,
        consecutive: bool,
    ) {
        let Some(id) = &frame.meter.identity else {
            return;
        };
        let (Some(serial), Some(manufacturer), Some(version), Some(device)) = (
            id.id.as_deref(),
            id.manufacturer_code.as_deref(),
            id.version,
            id.device_type_code,
        ) else {
            return;
        };
        let history = self
            .meters
            .entry((serial, manufacturer, version, device))
            .or_default();
        let static_push = matches!(frame.frame.kind.as_str(), "wireless" | "lpwan")
            && frame.frame.control_field.as_deref() == Some("44")
            && frame
                .transport
                .configuration
                .as_ref()
                .and_then(|cf| u16::from_str_radix(&cf.raw, 16).ok())
                .is_some_and(|raw| {
                    m_bus_oms_rules::telegram::static_content(ConfigurationField::from(raw))
                });
        history.observe(report, static_push, consecutive, |number| {
            let record = frame.records.get(number.checked_sub(1)?)?;
            Some(RegisterValue {
                subunit: record.subunit,
                tariff: record.tariff,
                data: crate::output::decode_hex_bytes(&record.data_hex).ok()?,
            })
        });
    }
}
