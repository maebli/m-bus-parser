use crate::telegram::{RuleFinding, RuleStatus, TelegramReport};
use alloc::collections::BTreeMap;
use alloc::vec::Vec;

#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub id: u8,
    pub control: u8,
    pub recipient: u8,
    pub selector: u8,
    pub key: u8,
    pub version: u8,
    body_len: usize,
    status: Option<u8>,
    der_sequence: bool,
}

fn der_sequence(data: &[u8]) -> bool {
    let [0x30, first, remaining @ ..] = data else {
        return false;
    };
    if *first < 128 {
        return usize::from(*first) == remaining.len();
    }
    let count = usize::from(first & 127);
    if count == 0 || count > size_of::<usize>() {
        return false;
    }
    let Some((encoded, body)) = remaining.split_at_checked(count) else {
        return false;
    };
    if encoded.first() == Some(&0) {
        return false;
    }
    let length = encoded
        .iter()
        .fold(0usize, |length, byte| (length << 8) | usize::from(*byte));
    length >= 128 && length == body.len()
}

impl Block {
    pub fn new([id, control, recipient, selector, key, version]: [u8; 6], body: &[u8]) -> Self {
        Self {
            id,
            control,
            recipient,
            selector,
            key,
            version,
            body_len: body.len(),
            status: body.first().copied(),
            der_sequence: der_sequence(body),
        }
    }

    fn selector_allowed(self) -> bool {
        match self.control {
            0 => matches!(self.selector, 1 | 0x10 | 0x11),
            1..=3 | 5 => self.selector == 2,
            4 => self.selector == 3,
            6..=9 => self.selector == 0,
            0x20 => matches!(self.selector, 0x30..=0x37),
            0x80..=0x85 => self.selector == 0x22,
            0x86 => matches!(self.selector, 0x10 | 0x12 | 0x13 | 0x22),
            0x87 => matches!(self.selector, 0x20 | 0x22),
            0x88 => matches!(self.selector, 0x21..=0x22),
            0x89 => matches!(self.selector, 0x22..=0x23),
            0xa0 => matches!(self.selector, 0x22 | 0x30..=0x37),
            0x70..=0x7f | 0xf0..=0xff => true,
            _ => false,
        }
    }
}

macro_rules! source {
    ($page:literal,$top:literal) => {
        concat!(
            "https://oms-group.org/wp-content/uploads/2025/07/OMS-Spec_Vol2_AnnexF_C111.pdf#page=",
            $page,
            "&view=FitH,",
            $top
        )
    };
}

pub fn assess(blocks: &[Block], command: bool, report: &mut TelegramReport) {
    let Some(first) = blocks.first() else { return };
    let mut check = |rule, reference, source_url, valid, detail| {
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
        })
    };
    if command {
        check(
            "First SITP block identifier",
            "Annex F.A.4",
            source!(37, 300),
            first.id == 0,
            "The first command block identifier must be zero.",
        );
    }
    let mut ids = [false; 256];
    let unique = blocks
        .iter()
        .all(|b| !core::mem::replace(&mut ids[usize::from(b.id)], true));
    check(
        "SITP block identifiers",
        "Annex F.A.4",
        source!(37, 300),
        unique,
        "Each block identifier must allow its response to be correlated to one command.",
    );
    check(
        "SITP block direction",
        "Annex F.A.5, Table F.A.3",
        source!(37, 547),
        blocks.iter().all(|b| (b.control < 0x80) == command),
        "The block control must be a command or response in the direction declared by its CI.",
    );
    check(
        "SITP block control and data structure",
        "Annex F.A.5, Table F.A.3",
        source!(37, 547),
        blocks.iter().all(|b| b.selector_allowed()),
        "Each block must use a data structure permitted for its command or response.",
    );
    for block in blocks {
        if matches!(block.control,0x70..=0x7f|0xf0..=0xff) {
            continue;
        }
        let reference = "Annex F.A.7, Table F.A.5";
        let url = source!(39, 101);
        match block.selector {
            0 => check(
                "SITP empty structure",
                reference,
                url,
                block.body_len == 0,
                "An empty structure must omit data content.",
            ),
            0x10..=0x13 => {
                check(
                    "SITP asymmetric key version",
                    reference,
                    url,
                    block.version == 0xff,
                    "The unused asymmetric-key version must be FF.",
                );
                check(
                    "SITP DER sequence envelope",
                    reference,
                    url,
                    block.der_sequence,
                    "A DER certificate, key pair, public key or certificate request must contain a complete definite-length SEQUENCE.",
                );
            }
            0x21 | 0x23 => check(
                "SITP unused key information",
                reference,
                url,
                block.key == 0xff && block.version == 0xff,
                "Both unused DSH fields must be FF.",
            ),
            0x22 => {
                check(
                    "SITP status response length",
                    "Annex F.A.8",
                    source!(41, 104),
                    block.body_len == 1,
                    "A status response must contain one status octet.",
                );
                if let Some(status) = block.status {
                    check(
                        "SITP reserved status code",
                        "Annex F.A.8, Table F.A.7",
                        source!(41, 104),
                        !matches!(status,0x0a..=0x0f|0x12..=0x13|0x82..=0x8f),
                        "A status response must not use an explicitly reserved status code.",
                    );
                }
            }
            0x30..=0x37 => check(
                "SITP application security key",
                reference,
                source!(40, 79),
                block.key > 0x0f && block.key != 0xff,
                "An application wrapper or authentication key must use an application key ID.",
            ),
            _ => {}
        }
        if block.selector == 0x23 {
            check(
                "SITP active key counter structure",
                "Annex F.A.8, Table F.A.8",
                source!(41, 535),
                block.body_len <= 256 * 6 && block.body_len.is_multiple_of(6),
                "An active key counter list contains at most 256 six-octet entries.",
            );
        }
        if crate::key_wrap::is_wrapped_structure(block.selector) {
            check(
                "AES-KWP container length",
                reference,
                url,
                crate::key_wrap::valid_wire_length(block.selector, block.body_len),
                "The selected structure sets its encoded length. An application container adds a four-byte external counter.",
            );
        }
    }
}

pub fn assess_asp20(verified: Option<bool>, report: &mut TelegramReport) {
    report.push(RuleFinding {
        rule: "ASP20 application authentication",
        reference: "Section 9.4.2.6, Table 47",
        source_url: "https://oms-group.org/wp-content/uploads/2024/10/OMS-Spec_Vol2_Primary_v501_01.pdf#page=97&view=FitH,89",
        status: match verified {
            Some(true) => RuleStatus::Passed,
            Some(false) => RuleStatus::Failed,
            None => RuleStatus::CouldNotCheck,
        },
        detail: "AES-CCM must authenticate the application payload using the meter address, direction, key counter and application key.",
        record_number: None,
    });
}

#[derive(Default)]
pub struct History {
    requests: BTreeMap<([u8; 8], [u8; 8]), Vec<Block>>,
}

impl History {
    pub fn observe(
        &mut self,
        sender: [u8; 8],
        receiver: [u8; 8],
        blocks: &[Block],
        command: bool,
        consecutive: bool,
        report: &mut TelegramReport,
    ) {
        if command {
            self.requests.insert((sender, receiver), blocks.to_vec());
            return;
        }
        let Some(request) = self.requests.remove(&(receiver, sender)) else {
            return;
        };
        let valid = request.len() == blocks.len()
            && request.iter().all(|a| {
                blocks.iter().any(|b| {
                    a.id == b.id
                        && a.control | 0x80 == b.control
                        && a.recipient == b.recipient
                        && a.key == b.key
                        && a.version == b.version
                })
            });
        report.push(RuleFinding{rule:"SITP command and response blocks",reference:"Annex F.A.4–6",source_url:source!(38,198),status:if !consecutive {RuleStatus::CouldNotCheck} else if valid {RuleStatus::Passed} else {RuleStatus::Failed},detail:"Each returned block must match its command identifier, operation, recipient and two data structure header octets.",record_number:None});
    }
}
