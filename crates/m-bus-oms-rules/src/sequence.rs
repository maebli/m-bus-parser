use crate::telegram::{RuleFinding, RuleStatus};
use alloc::collections::BTreeMap;

#[derive(Clone, Copy)]
pub struct AccessObservation {
    pub sender: [u8; 8],
    pub synchronous: Option<bool>,
    pub repeated: bool,
    pub number: Option<u8>,
}

#[derive(Default)]
pub struct AccessHistory {
    last_synchronous: BTreeMap<[u8; 8], u8>,
}

impl AccessHistory {
    pub fn observe(&mut self, frame: AccessObservation, consecutive: bool) -> Option<RuleFinding> {
        if frame.repeated {
            return None;
        }
        let previous = self.last_synchronous.get(&frame.sender).copied();
        if frame.synchronous != Some(false) {
            match (frame.synchronous, frame.number) {
                (Some(true), Some(number)) => {
                    self.last_synchronous.insert(frame.sender, number);
                }
                _ => {
                    self.last_synchronous.remove(&frame.sender);
                }
            }
        }
        let synchronous = frame.synchronous?;
        let expected = previous.map(|last| {
            if synchronous {
                last.wrapping_add(1)
            } else {
                last
            }
        });
        let (status, detail) = match (expected, frame.number) {
            (_, None) => (
                RuleStatus::CouldNotCheck,
                "The access number is unavailable.",
            ),
            (None, _) => (
                RuleStatus::CouldNotCheck,
                "An earlier synchronous telegram from this sender is needed.",
            ),
            (Some(expected), Some(number)) if expected == number => (
                RuleStatus::Passed,
                "The counter matches the preceding observed synchronous transmission.",
            ),
            _ if consecutive => (
                RuleStatus::Failed,
                "The counter differs from the value required in this consecutive capture.",
            ),
            _ => (
                RuleStatus::CouldNotCheck,
                "The counter differs. Confirm the capture contains every transmission since the previous synchronous telegram.",
            ),
        };
        Some(RuleFinding {
            rule: if synchronous {
                "Synchronous access number increment"
            } else {
                "Asynchronous access number reuse"
            },
            reference: "§7.2.2.1",
            source_url: primary!(59, 251),
            status,
            detail,
            record_number: None,
        })
    }
}

#[derive(Clone, Copy)]
pub struct ExchangeFrame {
    pub sender: [u8; 8],
    pub receiver: Option<[u8; 8]>,
    pub function: u8,
    pub link_access: Option<u8>,
    pub transport_access: Option<u8>,
    pub transport_status: Option<u8>,
    pub fragmented: bool,
    pub security_mode: Option<u8>,
    pub key_information_present: Option<bool>,
    pub security_counter: Option<u32>,
    pub key_id: Option<u8>,
    pub key_version: Option<u8>,
    pub verified_encryption: bool,
    pub tls_hello: Option<crate::tls_hello::TlsHello>,
}

#[derive(Default)]
pub struct KeyInformationHistory {
    requests: BTreeMap<[u8; 8], bool>,
    first_ell: BTreeMap<[u8; 8], bool>,
    tls_offers: BTreeMap<([u8; 8], [u8; 8]), bool>,
}

impl KeyInformationHistory {
    pub fn check_mode_c_ell(
        &mut self,
        current: ExchangeFrame,
        report: &mut crate::telegram::TelegramReport,
    ) {
        if !matches!(current.function, 0x44 | 0x46 | 0x47) {
            return;
        }
        let present = current.link_access.is_some();
        if let Some(first) = self.first_ell.get(&current.sender) {
            report.push(RuleFinding {
                rule: "Consistent Mode C ELL usage",
                reference: "§5.2.2.2.4",
                source_url: primary!(51, 549),
                status: if *first == present { RuleStatus::Passed } else { RuleStatus::Failed },
                detail: "An end device must use the ELL consistently across its SND-NR, SND-IR and ACC-NR transmissions.",
                record_number: None,
            });
        } else {
            self.first_ell.insert(current.sender, present);
        }
    }

    pub fn observe(
        &mut self,
        current: ExchangeFrame,
        consecutive: bool,
        report: &mut crate::telegram::TelegramReport,
    ) {
        if let (Some(hello), Some(receiver)) = (current.tls_hello, current.receiver) {
            if hello.client {
                self.tls_offers
                    .insert((current.sender, receiver), hello.encrypt_then_mac);
            } else if self.tls_offers.remove(&(receiver, current.sender)) == Some(true) {
                let valid = if hello.encrypt_then_mac {
                    Some(true)
                } else {
                    hello.selected_gcm
                };
                report.push(RuleFinding {
                    rule: "TLS server encrypt-then-MAC selection",
                    reference: "Annex F.3.1",
                    source_url: "https://oms-group.org/wp-content/uploads/2025/07/OMS-Spec_Vol2_AnnexF_C111.pdf#page=12&view=FitH,110",
                    status: match (valid, consecutive) {
                        (Some(true), true) => RuleStatus::Passed,
                        (Some(false), true) => RuleStatus::Failed,
                        _ => RuleStatus::CouldNotCheck,
                    },
                    detail: "When the client offers encrypt-then-MAC, the server must select GCM or negotiate encrypt-then-MAC.",
                    record_number: None,
                });
            }
        }
        if matches!(current.function, 0x43 | 0x53 | 0x73) {
            if current.fragmented {
                self.requests.clear();
            } else if let Some(address) = current.receiver {
                if let Some(present) = current.key_information_present {
                    self.requests.insert(address, present);
                } else {
                    self.requests.remove(&address);
                }
            } else {
                self.requests.clear();
            }
            return;
        }
        if !matches!(current.function, 0x08 | 0x18 | 0x28 | 0x38) || current.fragmented {
            return;
        }
        let requested = self.requests.get(&current.sender).copied();
        let mode_d = current.security_mode == Some(10);
        if !mode_d && requested != Some(true) {
            return;
        }
        let consistent = requested
            .zip(current.key_information_present)
            .map(|(a, b)| a == b);
        report.push(RuleFinding {
            rule: "Response key information flag",
            reference: "§6.2.2, §7.2.4.5",
            source_url: if mode_d { primary!(65, 471) } else { primary!(56, 477) },
            status: match consistent {
                Some(true) if consecutive => RuleStatus::Passed,
                Some(false) if consecutive => RuleStatus::Failed,
                _ => RuleStatus::CouldNotCheck,
            },
            detail: "The last addressed SND-UD or SND-UD2 sets the response key-information flag. Mode 10 uses the transport key-version flag.",
            record_number: None,
        });
    }
}

impl ExchangeFrame {
    fn check_security_counter(
        self,
        next: Self,
        consecutive: bool,
        report: &mut crate::telegram::TelegramReport,
    ) {
        if self.sender != next.sender
            || self.key_id != Some(0)
            || next.key_id != self.key_id
            || self.key_version != next.key_version
            || !self.verified_encryption
            || !next.verified_encryption
            || self
                .transport_access
                .zip(next.transport_access)
                .is_none_or(|(a, b)| a == b)
            || self.fragmented
            || next.fragmented
        {
            return;
        }
        let meter = matches!(self.function, 0x44 | 0x46) && matches!(next.function, 0x44 | 0x46);
        let gateway = matches!(self.function, 0x43 | 0x53 | 0x73)
            && matches!(next.function, 0x43 | 0x53 | 0x73)
            && self.receiver.is_some()
            && self.receiver == next.receiver;
        if !meter && !gateway {
            return;
        }
        let Some((prior, current)) = self.security_counter.zip(next.security_counter) else {
            return;
        };
        let delta = current.checked_sub(prior);
        let valid = if meter {
            delta == Some(1)
        } else {
            delta.is_some_and(|d| (1..=100).contains(&d))
        };
        report.push(RuleFinding {
            rule: "Authenticated message counter progression",
            reference: if meter { "§9.3.2.2" } else { "§9.3.2.4" },
            source_url: if meter { primary!(89, 690) } else { primary!(91, 314) },
            status: if !consecutive || (!valid && self.key_version.is_none() && current <= prior) {
                RuleStatus::CouldNotCheck
            } else if valid { RuleStatus::Passed } else { RuleStatus::Failed },
            detail: if meter { "Consecutive authenticated and encrypted end-device transmissions increment the counter by one." }
                else { "Consecutive authenticated and encrypted commands for the same end device increment the gateway counter by one to 100." },
            record_number: None,
        });
    }

    pub fn check_reply(
        self,
        reply: Self,
        consecutive: bool,
        report: &mut crate::telegram::TelegramReport,
    ) {
        self.check_security_counter(reply, consecutive, report);
        let ack = matches!(reply.function, 0x00 | 0x10 | 0x20 | 0x30);
        let nack = matches!(reply.function, 0x01 | 0x11 | 0x21 | 0x31);
        let response = matches!(reply.function, 0x08 | 0x18 | 0x28 | 0x38);
        let valid_pair = match self.function {
            0x46 => reply.function == 0x06,
            0x48 => reply.function == 0,
            0x53 | 0x73 => ack || nack,
            0x43 => response || nack,
            0x5a | 0x7a => ack || response,
            0x5b | 0x7b => response,
            _ => false,
        };
        if !valid_pair
            || self.sender == reply.sender
            || self.receiver.is_some_and(|peer| peer != reply.sender)
            || reply.receiver.is_some_and(|peer| peer != self.sender)
            || self.receiver.is_none() && reply.receiver.is_none()
        {
            return;
        }
        let mut check = |rule, reference, source_url, consistent, detail| {
            let status = match consistent {
                Some(true) => RuleStatus::Passed,
                Some(false) if consecutive => RuleStatus::Failed,
                _ => RuleStatus::CouldNotCheck,
            };
            report.push(RuleFinding {
                rule,
                reference,
                source_url,
                status,
                detail,
                record_number: None,
            });
        };
        let from_meter = matches!(self.function, 0x46 | 0x48);
        let request_number = self
            .link_access
            .or_else(|| from_meter.then_some(self.transport_access).flatten());
        let reply_number = reply
            .link_access
            .or_else(|| from_meter.then_some(reply.transport_access).flatten());
        check(
            "Reply link access number",
            "§7.2.2.1",
            if from_meter {
                primary!(59, 251)
            } else {
                primary!(60, 274)
            },
            request_number
                .zip(reply_number)
                .map(|(request, reply)| request == reply),
            "The addressed reply echoes the initiating access number. A mismatch needs a consecutive capture.",
        );
        if self.receiver.is_some() {
            check(
                "Reply destination address",
                "§3.1.3.3",
                primary!(31, 692),
                Some(reply.receiver == Some(self.sender)),
                "A reply to a telegram with an ELL destination must address its sender.",
            );
        }
        if self.fragmented || reply.fragmented {
            return;
        }
        if matches!(self.function, 0x53 | 0x73) && ack {
            check(
                "Acknowledgement transport header",
                "§7.3, Table 28",
                primary!(68, 111),
                Some(reply.transport_access.is_some()),
                "The ACK to an unfragmented wireless SND-UD requires a transport header.",
            );
        }
        if matches!(self.function, 0x5b | 0x7b) && response
            || matches!(self.function, 0x53 | 0x73) && ack
        {
            check(
                "Reply transport access number",
                "§7.2.2.1",
                primary!(59, 554),
                self.transport_access
                    .zip(reply.transport_access)
                    .map(|(a, b)| a == b),
                "The response to REQ-UD2 or acknowledgement of SND-UD echoes the request's TPL access number.",
            );
        }
        if matches!(self.function, 0x43 | 0x53 | 0x73) && (ack || response) {
            check(
                "Command response status",
                "§7.2.3, Table 17",
                primary!(60, 694),
                reply.transport_status.map(|status| status & 3 != 3),
                "A direct command response cannot use the abnormal-condition status.",
            );
        }
    }
}
