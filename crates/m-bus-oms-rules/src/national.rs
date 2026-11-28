use crate::layers::{LayerFields, OmsRadioMode};
use crate::telegram::{Link, RuleFinding, RuleStatus, TelegramReport};
use m_bus_core::ConfigurationField;

macro_rules! annex_e {
    ($page:literal, $top:literal) => {
        concat!(
            "https://oms-group.org/wp-content/uploads/2024/05/OMS-Spec_Vol2_AnnexE_B203.pdf#page=",
            $page,
            "&view=FitH,",
            $top
        )
    };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Deserialize),
    serde(rename_all = "kebab-case")
)]
pub enum OmsNationalProfile {
    BsiUnidirectional,
    BsiBidirectional,
    Idis,
}

impl core::str::FromStr for OmsNationalProfile {
    type Err = &'static str;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "bsi-unidirectional" => Ok(Self::BsiUnidirectional),
            "bsi-bidirectional" => Ok(Self::BsiBidirectional),
            "idis" => Ok(Self::Idis),
            _ => Err("expected bsi-unidirectional, bsi-bidirectional or idis"),
        }
    }
}

impl OmsNationalProfile {
    pub fn check(
        self,
        link: Link,
        lpwan: bool,
        fields: LayerFields,
        cf: Option<ConfigurationField>,
        clock: bool,
        report: &mut TelegramReport,
    ) {
        let wireless = matches!(link, Link::Wireless(_));
        if matches!(link, Link::Unavailable) && !lpwan {
            return;
        }
        let (reference, source_url) = match (self, wireless || lpwan) {
            (Self::Idis, true) => ("Annex E, Table E.2.1", annex_e!(12, 646)),
            (Self::Idis, false) => ("Annex E, Table E.2.2", annex_e!(13, 103)),
            (_, true) => ("Annex E, Table E.1.3", annex_e!(8, 266)),
            (_, false) => ("Annex E, Table E.1.4", annex_e!(9, 103)),
        };
        let mut check = |rule, valid, detail| {
            report.push(RuleFinding {
                rule,
                reference,
                source_url,
                status: if valid {
                    RuleStatus::Passed
                } else {
                    RuleStatus::Failed
                },
                detail,
                record_number: None,
            });
        };
        if lpwan {
            check(
                "National profile radio interface",
                false,
                "The selected Annex E meter profile specifies conventional wireless M-Bus modes. This telegram uses OMS LPWAN.",
            );
            return;
        }
        if self != Self::Idis
            && wireless
            && let Some(mode @ (OmsRadioMode::C1 | OmsRadioMode::C2)) = fields.radio_mode
        {
            check(
                "National profile radio direction",
                matches!(
                    (self, mode),
                    (Self::BsiUnidirectional, OmsRadioMode::C1)
                        | (Self::BsiBidirectional, OmsRadioMode::C2)
                ),
                "The declared C1 or C2 radio mode must agree with the selected meter interface.",
            );
        }
        if self != Self::Idis && clock {
            check(
                "National profile clock service",
                false,
                "The BSI meter profile excludes the OMS clock synchronisation protocol.",
            );
        }
        if !matches!(
            fields.application_ci.or(fields.outer_ci),
            Some(0x72 | 0x78 | 0x7a)
        ) {
            return;
        }
        let mode = cf.map_or(0, |c| c.security_mode().to_bits());
        let is_push = matches!(link, Link::Wireless(0x44));
        let valid = match (self, wireless) {
            (Self::Idis, true) => matches!(mode, 5 | 7),
            (Self::Idis, false) => matches!(mode, 0 | 5 | 7),
            (Self::BsiUnidirectional, true) => mode == 7,
            (Self::BsiBidirectional, true) if is_push => mode == 7,
            _ => mode == 13,
        };
        check(
            "National profile data protection",
            valid,
            "The protection mode of a meter-data telegram must match the declared interface. A BSI profile C meter uses profile B for its unsolicited consumption fallback.",
        );
        if self != Self::Idis && wireless {
            check(
                "National profile extended link layer",
                fields.communication_control.is_some(),
                "The BSI wireless meter profile requires the extended link layer for wireless meter data.",
            );
            if let Some(device) = fields.device_type {
                check(
                    "National profile meter type",
                    matches!(device, 2..=4 | 6..=8 | 0x0a..=0x0d | 0x15..=0x16 | 0x20..=0x21 | 0x28),
                    "The BSI wireless meter profile restricts the device type to Table 2 of Volume 2 v4.3.x, as referenced by Annex E.",
                );
            }
            if mode == 7 {
                check(
                    "National profile data authentication",
                    fields.authentication_type.is_some(),
                    "The profile B path requires AFL authentication.",
                );
            }
        }
    }
}
