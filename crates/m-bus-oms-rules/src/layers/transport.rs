use alloc::vec::Vec;
use m_bus_core::{ConfigurationField, DeviceType, Function};

use super::{LayerFields, finding, uplink_ci};
use crate::telegram::{Link, RuleFinding};

impl LayerFields {
    pub(super) fn check_transport_header(self, link: Link, results: &mut Vec<RuleFinding>) {
        let function = link.function();
        if matches!(link, Link::Wired(_)) && self.tpl_present == Some(true) {
            results.push(finding(
                "Wired transport header",
                ("§7.2.1", primary!(58, 464)),
                Some(!self.short_tpl),
                "A wired telegram with a TPL header requires the long form.",
            ));
        }
        let required_tpl = match (link, function) {
            (Link::Wireless(0x00), _)
                if matches!(
                    self.wireless_sender_type.map(DeviceType::from),
                    Some(
                        DeviceType::CommunicationController | DeviceType::RadioConverterSystemSide
                    )
                ) =>
            {
                Some(true)
            }
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
            (Link::Wireless(_), Some(Function::AccNr))
                if self.communication_control.is_none() && self.lpwan_sync.is_none() =>
            {
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
    }

    pub(super) fn check_ci_fields(
        self,
        link: Link,
        direction: Option<bool>,
        results: &mut Vec<RuleFinding>,
    ) {
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
                && self.lpwan_sync.is_none()
                && matches!(ci, 0x50..=0x52 | 0x54 | 0x56..=0x57 | 0x5a | 0x61..=0x62 | 0x65..=0x66 | 0x70..=0x71 | 0x92..=0x93 | 0xb8 | 0xbb | 0xbd..=0xbf | 0xc6..=0xc7 | 0xcf)
            {
                results.push(finding(
                    "Radio mode restriction",
                    ("§2.2, Table 1 note d", primary!(25, 107)),
                    self.radio_mode.map(|mode| !mode.is_c()),
                    "This CI is prohibited in radio mode C.",
                ));
            }
        }
    }

    pub(super) fn check_configuration(
        self,
        link: Link,
        cf: ConfigurationField,
        direction: Option<bool>,
        has_afl: Option<bool>,
        results: &mut Vec<RuleFinding>,
    ) {
        self.check_security_fields(cf, results);
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
        if mode == 13 && matches!(link, Link::Wireless(_)) && self.lpwan_sync.is_none() {
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
