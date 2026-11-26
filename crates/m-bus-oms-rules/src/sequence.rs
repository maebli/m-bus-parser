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
}

impl ExchangeFrame {
    pub fn check_reply(
        self,
        reply: Self,
        consecutive: bool,
        report: &mut crate::telegram::TelegramReport,
    ) {
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
