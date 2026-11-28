use alloc::vec::Vec;
use m_bus_core::{ConfigurationField, Function};

use super::{LayerFields, OmsRadioMode, finding, uplink_ci};
use crate::telegram::{Link, RuleFinding, RuleStatus};

impl LayerFields {
    fn check_radio(
        self,
        link: Link,
        configuration: Option<ConfigurationField>,
        results: &mut Vec<RuleFinding>,
    ) {
        if let Some(mode) = self.ell_encryption_mode {
            results.push(finding(
                "Wireless link-layer encryption",
                ("§5.2.1", primary!(49, 296)),
                Some(mode == 0),
                "OMS wireless M-Bus prohibits ELL encryption.",
            ));
        }
        if let Some(cc) = self.communication_control {
            results.push(finding("Original ELL transmission flags", ("§5.2.2.2.3", primary!(51, 455)),
                (cc & 0x10 == 0 && (!matches!(link, Link::Wireless(0x44 | 0x46..=0x48 | 8 | 0x18 | 0x28 | 0x38)) || cc & 2 == 0)).then_some(true),
                "An original transmission clears the hop bit and an originating end device clears the repeated-access bit. A marked capture may come from a repeater."));
        }
        if !self.radio_mode.is_some_and(OmsRadioMode::is_c) {
            return;
        }
        let ell = self.communication_control.is_some();
        let legacy = matches!(link, Link::Wireless(0x44 | 0x46 | 0x47))
            && configuration.is_some_and(|cf| matches!(cf.security_mode().to_bits(), 0 | 5));
        let permitted = match (ell, legacy, self.radio_mode) {
            (true, _, _) | (false, true, Some(OmsRadioMode::C1)) => Some(true),
            (false, true, Some(OmsRadioMode::C)) => None,
            _ => Some(false),
        };
        results.push(finding("Mode C extended link layer", ("§5.2.2.2.4", primary!(51, 549)), permitted,
            "Mode C requires an ELL. A unidirectional device using only modes 0 and 5 may omit it for SND-NR, SND-IR and ACC-NR."));
        if self.radio_mode == Some(OmsRadioMode::C1)
            && matches!(link, Link::Wireless(0x44 | 0x46 | 0x47))
        {
            let flags = self
                .communication_control
                .map(|cc| cc & 0x84 != 0)
                .or_else(|| {
                    configuration
                        .filter(|cf| matches!(cf.security_mode().to_bits(), 0 | 5))
                        .map(|cf| cf.raw() & 0xc000 != 0)
                });
            results.push(finding(
                "Mode C1 accessibility",
                ("§4.3.3.1, Table 10", primary!(45, 114)),
                flags.map(|available| !available),
                "A unidirectional device must advertise no access windows.",
            ));
        }
    }

    pub(super) fn check_link(
        self,
        link: Link,
        configuration: Option<ConfigurationField>,
        direction: Option<bool>,
        results: &mut Vec<RuleFinding>,
    ) {
        if self.lpwan_sync.is_none() && matches!(link, Link::Wireless(_)) {
            self.check_radio(link, configuration, results);
        }
        let synchronous = self
            .lpwan_sync
            .or_else(|| self.communication_control.map(|cc| cc & 0x20 != 0))
            .or_else(|| {
                configuration
                    .filter(|cf| matches!(cf.security_mode().to_bits(), 0 | 5))
                    .map(|cf| cf.raw() & 0x2000 != 0)
            });
        if matches!(
            link,
            Link::Wireless(
                0x46 | 0x08 | 0x18 | 0x28 | 0x38 | 0x01 | 0x11 | 0x21 | 0x31 | 0x10 | 0x20 | 0x30
            )
        ) {
            results.push(finding(
                "Asynchronous reply or installation",
                ("§4.3.2.1", primary!(42, 164)),
                synchronous.map(|sync| !sync),
                "A meter reply or installation telegram must use asynchronous transmission.",
            ));
        }
        if matches!(link, Link::Wireless(0x47)) {
            results.push(finding(
                "ACC-NR application data",
                ("§8.2.3, Table 33", primary!(71, 510)),
                self.has_application_data.map(|present| !present),
                "An ACC-NR signals access and carries no application data.",
            ));
        }
        if matches!(link, Link::Wireless(_))
            && matches!(link.function(), Some(Function::AccDmd | Function::Ack))
        {
            results.push(finding(
                "Control message application data",
                ("§5.2.4, Tables 15–16", primary!(53, 422)),
                self.has_application_data.map(|present| !present),
                "ACC-DMD and ACK messages must omit application data.",
            ));
        }
        let adapter = match direction {
            Some(true) => self.wireless_sender_type,
            Some(false) => self.wireless_receiver_type,
            None => None,
        };
        if adapter == Some(0x37)
            && self.application_ci.is_some_and(|ci| {
                !matches!(ci, 0x8a | 0x8b | 0x93)
                    && (uplink_ci(ci).is_some() || matches!(ci, 0x73 | 0x76..=0x79))
            })
        {
            results.push(finding("RF adapter application address", ("§3.1.3.4", primary!(32, 498)),
                self.tpl_present.map(|present| present && !self.short_tpl), "Application messages sent through an external meter-side RF adapter require the application address in a long TPL header."));
        }
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
    }

    pub(super) fn check_ell_control(self, results: &mut Vec<RuleFinding>) {
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
    }
}
