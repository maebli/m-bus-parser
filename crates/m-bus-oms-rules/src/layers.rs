mod link;
mod security;
mod transport;

use alloc::vec::Vec;
use m_bus_core::{ConfigurationField, Function};

use crate::telegram::{Link, RuleFinding, RuleStatus};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(rename_all = "lowercase")
)]
pub enum OmsRadioMode {
    C,
    C1,
    C2,
    Other,
}

impl OmsRadioMode {
    pub fn is_c(self) -> bool {
        matches!(self, Self::C | Self::C1 | Self::C2)
    }
}

#[derive(Clone, Copy, Default)]
pub struct LayerFields {
    pub application_protocol: crate::telegram::ApplicationProtocolEvidence,
    pub device_type: Option<u8>,
    pub command_target_type: Option<u8>,
    pub wireless_sender_type: Option<u8>,
    pub wireless_receiver_type: Option<u8>,
    pub has_application_data: Option<bool>,
    pub mac_verified: bool,
    pub verification_prefix: Option<bool>,
    pub tpl_present: Option<bool>,
    pub short_tpl: bool,
    pub outer_ci: Option<u8>,
    pub application_ci: Option<u8>,
    pub communication_control: Option<u8>,
    pub configuration_extension: Option<u16>,
    pub layers_decoded: bool,
    pub authentication_type: Option<u8>,
    pub message_length: Option<u16>,
    pub reassembled: bool,
    pub lpwan_sync: Option<bool>,
    pub radio_mode: Option<OmsRadioMode>,
    pub ell_encryption_mode: Option<u8>,
    pub security_counter: Option<u32>,
    pub transport_body_length: Option<usize>,
    pub tls: Option<TlsFields>,
}

#[derive(Clone, Copy)]
pub struct ClockFields {
    pub control: u8,
    pub length: usize,
    pub key: u8,
    pub block_control: u8,
    pub immediate: bool,
    pub verified: bool,
}

#[derive(Clone, Copy, Default)]
pub struct TlsFields {
    pub cmac_verified: bool,
    pub channel_request: bool,
    pub handshake_or_alert: bool,
    pub application: bool,
    pub alert: bool,
    pub clear_alert: bool,
    pub hello: Option<crate::tls_hello::TlsHello>,
}

fn finding(
    rule: &'static str,
    (reference, source_url): (&'static str, &'static str),
    valid: Option<bool>,
    detail: &'static str,
) -> RuleFinding {
    RuleFinding {
        rule,
        reference,
        source_url,
        status: match valid {
            Some(true) => RuleStatus::Passed,
            Some(false) => RuleStatus::Failed,
            None => RuleStatus::CouldNotCheck,
        },
        detail,
        record_number: None,
    }
}

fn uplink_ci(ci: u8) -> Option<bool> {
    match ci {
        0x50..=0x57
        | 0x5a..=0x5b
        | 0x5f..=0x62
        | 0x64..=0x65
        | 0x6c..=0x6d
        | 0x80
        | 0x82
        | 0x92..=0x93
        | 0xb8
        | 0xbb
        | 0xbd..=0xc0
        | 0xc3
        | 0xc6..=0xc7 => Some(false),
        0x66..=0x68
        | 0x6e..=0x72
        | 0x74..=0x75
        | 0x7a
        | 0x7c..=0x7f
        | 0x87..=0x88
        | 0x8a..=0x8b
        | 0x9e..=0x9f
        | 0xc1..=0xc2
        | 0xc4..=0xc5 => Some(true),
        _ => None,
    }
}

impl LayerFields {
    pub(super) fn readings_possible(&self) -> bool {
        self.application_ci.or(self.outer_ci).and_then(uplink_ci) != Some(false)
    }

    pub(super) fn content_index(self, cf: ConfigurationField) -> Option<u8> {
        match cf.security_mode().to_bits() {
            7 => Some(cf.mode_specific_lower() & 7),
            10 => self
                .configuration_extension
                .map(|ext| ((ext >> 10) & 15) as u8),
            _ => None,
        }
    }

    pub(super) fn standard_push(self, link: Link, cf: Option<ConfigurationField>) -> Option<bool> {
        if !matches!(link, Link::Wireless(0x44)) {
            return Some(false);
        }
        let cf = cf?;
        let mode = cf.security_mode().to_bits();
        let synchronous = self
            .lpwan_sync
            .or_else(|| self.communication_control.map(|cc| cc & 0x20 != 0))
            .or_else(|| (mode == 5).then_some(cf.raw() & 0x2000 != 0));
        match mode {
            5 if cf.mode_specific_lower() & 12 == 0 => synchronous,
            13 if cf.raw() >> 14 == 0 => synchronous,
            7 | 10 if cf.raw() >> 14 == 0 => match self.content_index(cf)? {
                0 => synchronous,
                index => Some(index == 2),
            },
            _ => Some(false),
        }
    }

    pub(super) fn check(
        self,
        link: Link,
        configuration: Option<ConfigurationField>,
        results: &mut Vec<RuleFinding>,
    ) {
        let function = link.function();
        let direction = match function {
            Some(
                Function::SndUd { .. }
                | Function::SndUd2
                | Function::SndNk { .. }
                | Function::ReqUd1 { .. }
                | Function::ReqUd2 { .. }
                | Function::CnfIr,
            ) => Some(false),
            Some(
                Function::RspUd { .. }
                | Function::SndNr
                | Function::SendIr
                | Function::AccNr
                | Function::AccDmd
                | Function::Nack,
            ) => Some(true),
            _ => None,
        };
        self.check_link(link, configuration, direction, results);
        self.check_transport_header(link, results);
        self.check_ci_fields(link, direction, results);
        self.check_ell_control(results);
        let has_afl = self.afl_present();
        self.check_authentication(link, configuration, has_afl, results);
        if let Some(cf) = configuration {
            self.check_configuration(link, cf, direction, has_afl, results);
        }
    }
}
