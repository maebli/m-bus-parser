use alloc::vec::Vec;
use m_bus_core::{ConfigurationField, Function};

use crate::telegram::{Link, RuleFinding, RuleStatus};

#[derive(Clone, Copy, Default)]
pub struct LayerFields {
    pub device_type: Option<u8>,
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

    fn content_index(self, cf: ConfigurationField) -> Option<u8> {
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
            .communication_control
            .map(|cc| cc & 0x20 != 0)
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
        if let Some(device) = self.device_type {
            let covered = matches!(device, 0x00 | 0x02..=0x04 | 0x06..=0x08 | 0x0a..=0x0d | 0x15..=0x16 | 0x18 | 0x1a | 0x1d..=0x21 | 0x25 | 0x28 | 0x31..=0x33 | 0x36..=0x38);
            results.push(RuleFinding {
                rule: "Device type in scope",
                reference: "§2.3, Tables 2–4",
                source_url: if covered {
                    primary!(27, 86)
                } else {
                    primary!(28, 86)
                },
                status: if covered {
                    RuleStatus::Passed
                } else {
                    RuleStatus::NotApplicable
                },
                detail: if covered {
                    "The device type is covered by the specification."
                } else {
                    "Table 4 device types fall outside OMS conformance testing."
                },
                record_number: None,
            });
        }
        if matches!(link, Link::Wired(_)) && self.tpl_present == Some(true) {
            results.push(finding(
                "Wired transport header",
                ("§7.2.1", primary!(58, 464)),
                Some(!self.short_tpl),
                "A wired telegram with a TPL header requires the long form.",
            ));
        }
        let required_tpl = match (link, function) {
            (Link::Wireless(_), Some(Function::Nack)) => Some(false),
            (
                Link::Wired(_),
                Some(
                    Function::ReqUd1 { .. }
                    | Function::ReqUd2 { .. }
                    | Function::SndNk { .. }
                    | Function::Ack,
                ),
            ) => Some(false),
            (
                Link::Wireless(_),
                Some(
                    Function::SndNr
                    | Function::SendIr
                    | Function::SndUd2
                    | Function::SndNk { .. }
                    | Function::CnfIr
                    | Function::AccDmd
                    | Function::ReqUd1 { .. },
                ),
            ) => Some(true),
            (Link::Wireless(_), Some(Function::AccNr)) if self.communication_control.is_none() => {
                Some(true)
            }
            (_, Some(Function::RspUd { .. }))
            | (Link::Wireless(_), Some(Function::ReqUd2 { .. } | Function::SndUd { .. }))
                if self.layers_decoded =>
            {
                Some(true)
            }
            _ => None,
        };
        let possible_continuation = matches!(link, Link::Wireless(_))
            && matches!(
                function,
                Some(Function::ReqUd2 { .. } | Function::SndUd { .. } | Function::RspUd { .. })
            )
            && self.tpl_present == Some(false)
            && self.outer_ci == Some(0x8e);
        if possible_continuation {
            results.push(finding("Transport header for link function", ("§7.3, Table 28 note b", primary!(68, 111)), None, "The first datagram requires a TPL. The fragment sequence is needed to establish its position."));
        } else if let Some(required) = required_tpl {
            results.push(finding(
                "Transport header for link function",
                ("§7.3, Table 28", primary!(68, 111)),
                self.tpl_present.map(|p| p == required),
                if required {
                    "This message requires a transport header."
                } else {
                    "This message must omit the transport header."
                },
            ));
        }
        let ci_fields = self
            .outer_ci
            .into_iter()
            .chain(self.application_ci.filter(|ci| Some(*ci) != self.outer_ci));
        for ci in ci_fields {
            let specified_direction = uplink_ci(ci);
            let listed = specified_direction.is_some() || matches!(ci, 0x8c | 0x8e | 0x90 | 0xcf);
            results.push(finding(
                if Some(ci) == self.outer_ci {
                    "Outer CI field"
                } else {
                    "Application CI field"
                },
                ("§2.2, Table 1", crate::sources::ci_row(ci)),
                Some(listed),
                "The CI field must appear in the OMS list.",
            ));
            if let Some(up) = specified_direction {
                results.push(finding(
                    "CI transmission direction",
                    ("§2.2, Table 1", crate::sources::ci_row(ci)),
                    direction.map(|d| d == up),
                    if up {
                        "This CI requires a message from the end device."
                    } else {
                        "This CI requires a message to the end device."
                    },
                ));
            }
            if matches!(link, Link::Wired(_)) {
                let wireless_only = matches!(
                    ci,
                    0x56 | 0x57
                        | 0x5a
                        | 0x61
                        | 0x62
                        | 0x65
                        | 0x82
                        | 0x87
                        | 0x88
                        | 0x8c
                        | 0x8e
                        | 0x92
                        | 0x93
                        | 0xc6
                        | 0xc7
                        | 0xcf
                );
                results.push(finding(
                    "CI on wired M-Bus",
                    ("§2.2, Table 1 note e", primary!(25, 107)),
                    Some(!wireless_only),
                    "This CI field must be permitted on the wired link.",
                ));
            }
            if matches!(link, Link::Wireless(_))
                && matches!(ci, 0x50..=0x52 | 0x54 | 0x56..=0x57 | 0x5a | 0x61..=0x62 | 0x65..=0x66 | 0x70..=0x71 | 0x92..=0x93 | 0xb8 | 0xbb | 0xbd..=0xbf | 0xc6..=0xc7 | 0xcf)
            {
                results.push(finding("Radio mode restriction", ("§2.2, Table 1 note d", primary!(25, 107)), None, "This CI is prohibited in mode C. The capture does not identify the radio mode."));
            }
        }
        if let Some(cc) = self.communication_control {
            for (name, mask) in [("ELL priority bit", 0x08), ("ELL reserved bit", 0x01)] {
                results.push(finding(
                    name,
                    ("§5.2.2.2.3, Table 14", primary!(51, 136)),
                    Some(cc & mask == 0),
                    "The communication-control bit must be zero.",
                ));
            }
        }
        let has_afl = if self.authentication_type.is_some()
            || self.outer_ci == Some(0x90)
            || self.reassembled
        {
            Some(true)
        } else if self.layers_decoded
            || self
                .outer_ci
                .is_some_and(|ci| !matches!(ci, 0x8c..=0x90 | 0xcf))
        {
            Some(false)
        } else {
            None
        };
        if function.is_some_and(crate::requires_mode_zero) {
            results.push(finding(
                "AFL on control messages",
                ("Table 43 note d", primary!(83, 145)),
                has_afl.map(|present| !present),
                "This link function requires a telegram without an AFL.",
            ));
        }
        if let Some(at) = self.authentication_type {
            results.push(finding(
                "AFL authentication type",
                ("§6.2.1", primary!(56, 321)),
                Some(at == 5),
                "Authenticated transmissions require AT 5 (eight-byte AES-CMAC).",
            ));
            if !self.reassembled {
                results.push(finding(
                    "Unfragmented AFL length field",
                    ("§6.2.5", primary!(56, 855)),
                    Some(self.message_length.is_none()),
                    "An unfragmented message must omit AFL.ML.",
                ));
            }
        }
        if let Some(length) = self.message_length {
            results.push(finding(
                "AFL message length",
                ("§6.2.5", primary!(56, 855)),
                Some(length <= 16384),
                "The message length may be at most 16 KiB.",
            ));
        }
        let Some(cf) = configuration else { return };
        let mode = cf.security_mode().to_bits();
        if direction == Some(true) && matches!(mode, 7 | 10) && cf.raw() >> 14 <= 1 {
            results.push(finding(
                "Message content index",
                (
                    "§7.2.4.7, Tables 26–27",
                    if cf.raw() >> 14 == 0 {
                        primary!(66, 629)
                    } else {
                        primary!(66, 871)
                    },
                ),
                self.content_index(cf).map(|idx| {
                    if cf.raw() >> 14 == 0 {
                        idx < 8
                    } else {
                        idx & 4 == 0
                    }
                }),
                "The content index must be listed for the declared message content.",
            ));
        }
        if matches!(mode, 7 | 10) {
            results.push(finding(
                "Message authentication",
                ("§9.3.6, Table 43", primary!(83, 145)),
                self.mac_verified.then_some(true),
                if self.mac_verified {
                    "The supplied key verified the authentication tag."
                } else {
                    "A verified decryption is required to check the authentication tag."
                },
            ));
        }
        if mode == 7 || (mode == 5 && cf.is_encrypted()) {
            results.push(finding(
                "Decryption verification bytes",
                (
                    "§7.2.4.3–4",
                    if mode == 5 {
                        primary!(62, 257)
                    } else {
                        primary!(63, 111)
                    },
                ),
                self.verification_prefix,
                "The decrypted transport payload must start with 2F 2F.",
            ));
        }
        let cf_page = match mode {
            7 => primary!(63, 111),
            10 => primary!(64, 205),
            _ => primary!(65, 471),
        };
        let ext_page = match mode {
            7 => primary!(63, 573),
            10 => primary!(64, 670),
            _ => primary!(66, 86),
        };
        let ext = self.configuration_extension;
        if matches!(mode, 7 | 10 | 13) {
            results.push(finding(
                "Transport configuration bit 13",
                ("Tables 20, 22, 24", cf_page),
                Some((cf.raw() & 0x2000 != 0) == (mode == 10)),
                if mode == 10 {
                    "Mode 10 requires the four-byte message counter flag."
                } else {
                    "The reserved bit must be zero."
                },
            ));
            results.push(finding(
                "Transport configuration extension",
                ("§7.2.4.4–6", ext_page),
                Some(ext.is_some()),
                "This security mode requires a configuration extension.",
            ));
        }
        if mode == 7 {
            results.push(finding(
                "Profile B authentication layer",
                ("§6.3", primary!(57, 91)),
                has_afl,
                "Security profile B requires an AFL.",
            ));
            results.push(finding(
                "Profile B message counter",
                ("§6.2.3", primary!(56, 622)),
                if self.authentication_type.is_some() {
                    Some(true)
                } else {
                    has_afl.filter(|v| !v)
                },
                "The profile B message requires AFL.MCR.",
            ));
        }
        if matches!(mode, 0 | 5) && self.authentication_type.is_some() {
            results.push(finding(
                "Profile authentication combination",
                ("Table 43", primary!(83, 145)),
                Some(false),
                "Mode 0 and profile A specify no MAC.",
            ));
        }
        if mode == 13 && matches!(link, Link::Wireless(_)) {
            results.push(finding(
                "Wireless TLS link layer",
                ("§7.2.4.6", primary!(66, 86)),
                Some(self.communication_control.is_some()),
                "Wireless mode 13 requires an ELL.",
            ));
        }
        if let Some(ext) = ext {
            if matches!(mode, 7 | 10) {
                results.push(finding(
                    "Key derivation function",
                    ("Tables 21, 23", ext_page),
                    Some(ext & 0x30 == 0x10),
                    "The key derivation field must be 01.",
                ));
            }
            let mask = match mode {
                7 => 0xc0,
                10 => 0xc080,
                13 => 0xf0,
                _ => 0,
            };
            if mask != 0 {
                results.push(finding(
                    "Reserved extension bits",
                    ("Tables 21, 23, 25", ext_page),
                    Some(ext & mask == 0),
                    "All reserved bits in the configuration extension must be zero.",
                ));
            }
            if mode == 10 {
                results.push(finding(
                    "CCM authentication tag size",
                    ("§7.2.4.5", primary!(65, 471)),
                    Some(ext & 0x300 == 0x100),
                    "Profile D requires an eight-byte authentication tag.",
                ));
            }
        }
    }
}
