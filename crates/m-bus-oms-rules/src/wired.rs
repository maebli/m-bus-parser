use crate::telegram::{RuleFinding, RuleStatus, TelegramReport};

#[derive(Clone, Copy)]
pub enum WiredObservation {
    Ack,
    Frame { control: u8, address: u8 },
}

fn finding(
    rule: &'static str,
    reference: &'static str,
    source_url: &'static str,
    valid: bool,
    detail: &'static str,
) -> RuleFinding {
    RuleFinding {
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
    }
}

impl WiredObservation {
    pub fn check(
        self,
        has_ci: bool,
        long_address: bool,
        encrypted: bool,
        report: &mut TelegramReport,
    ) {
        let Self::Frame { control, address } = self else {
            return;
        };
        let from_slave = control & 0x40 == 0;
        if from_slave {
            report.push(finding("Wired response primary address", "§3.1.2.1", primary!(29, 834), address <= 250, "The slave must reply with its own primary address, even after a secondary or broadcast request."));
        }
        if has_ci && (from_slave || encrypted) {
            report.push(finding("Wired application address", "§3.1.2.2", primary!(30, 450), long_address, "A slave message and an encrypted master message must provide the end-device application address."));
        }
    }

    pub fn check_reply(self, reply: Self, report: &mut TelegramReport) {
        let Self::Frame { control, address } = self else {
            return;
        };
        if control & 0x40 == 0 {
            return;
        }
        let slave_address = match reply {
            Self::Ack => None,
            Self::Frame { control, address } if control & 0x40 == 0 => Some(address),
            _ => return,
        };
        if address == 0xff {
            report.push(finding("Wired silent broadcast", "Annex P, §P.2.2.3", "https://oms-group.org/wp-content/uploads/2024/05/OMS-Spec_Vol2_AnnexP_C113.pdf#page=10", false, "A request to FFh suppresses the slave response."));
        } else if address <= 250
            && let Some(slave_address) = slave_address
        {
            report.push(finding(
                "Wired response source",
                "§3.1.2.1",
                primary!(29, 834),
                slave_address == address,
                "The primary address of the slave reply must match the addressed slave.",
            ));
        }
    }
}
