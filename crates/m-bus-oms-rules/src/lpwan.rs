use alloc::vec::Vec;

use crate::telegram::{OmsReport, RuleFinding, RuleStatus};

macro_rules! annex_q {
    ($page:literal, $top:literal) => {
        concat!(
            "https://oms-group.org/wp-content/uploads/2026/03/OMS-Spec_Vol2_AnnexQ_B201.pdf#page=",
            $page,
            "&view=FitH,",
            $top
        )
    };
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MacBody {
    pub control: u8,
    pub extension: u8,
    pub derivation_counter: Option<u8>,
    pub message_counter: Option<u16>,
    pub authentication_code: Option<[u8; 4]>,
    pub encrypted_blocks: Vec<u8>,
    pub authenticated: bool,
    pub blocks: Vec<MacBlock>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MacBlock {
    pub id: u8,
    pub header: u8,
    pub extension: u8,
    pub value: Vec<u8>,
}

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Llc {
    pub flags: u8,
    pub extension: u8,
    pub control: Option<u8>,
    pub sender: Option<[u8; 8]>,
    pub receiver: Option<[u8; 8]>,
    pub access_number: Option<u8>,
    pub run_time_delay: Option<u16>,
    pub adapter_status: Option<u8>,
}

#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct LpwanHeader {
    pub header: u8,
    pub extension: u8,
    pub elements: Vec<u8>,
    pub body: Option<MacBody>,
    pub llc: Option<Llc>,
    pub upper_layer_octets: usize,
}

pub fn has_mac_crc(frame: &[u8]) -> bool {
    let Some((data, expected)) = frame
        .len()
        .checked_sub(4)
        .and_then(|n| frame.split_at_checked(n))
    else {
        return false;
    };
    if data.is_empty() {
        return false;
    }
    let mut crc = 0u32;
    for byte in data {
        crc ^= u32::from(*byte) << 24;
        for _ in 0..8 {
            crc = if crc & 0x8000_0000 != 0 {
                (crc << 1) ^ 0xf4ac_fb13
            } else {
                crc << 1
            };
        }
    }
    expected == crc.to_be_bytes()
}

impl MacBlock {
    fn valid_value(&self, kind: u8) -> bool {
        let n = self.value.len();
        if matches!(kind, 12 | 13) && n == 0 {
            return !matches!(self.id, 3 | 7);
        }
        if kind == 1 && n == 0 {
            return true;
        }
        if matches!(kind, 12 | 13) && matches!(self.id, 0 | 1 | 7 | 0x10 | 0x11 | 0x13 | 0x14) {
            return false;
        }
        match self.id {
            0 => (1..=2).contains(&n) && self.value[0] & 0xf8 == 0,
            1 | 3 | 6 | 0x12 => n == 1,
            2 | 0x11 => n == 2,
            4 => n == 3 || (n == 1 && kind != 0),
            5 => (1..=2).contains(&n),
            7 => n == 2 && kind == 0,
            0x10 => (3..=8).contains(&n),
            0x13 => (2..=3).contains(&n),
            0x14 => (1..=3).contains(&n),
            _ => true,
        }
    }
}

fn take<'a>(input: &mut &'a [u8], n: usize) -> Result<&'a [u8], &'static str> {
    let (head, tail) = input
        .split_at_checked(n)
        .ok_or("Truncated LPWAN MAC or LLC field")?;
    *input = tail;
    Ok(head)
}

fn field<const N: usize>(input: &mut &[u8]) -> Result<[u8; N], &'static str> {
    take(input, N)?
        .try_into()
        .map_err(|_| "Truncated LPWAN field")
}

fn byte(input: &mut &[u8]) -> Result<u8, &'static str> {
    Ok(field::<1>(input)?[0])
}

fn optional<const N: usize>(
    flags: u8,
    mask: u8,
    input: &mut &[u8],
) -> Result<Option<[u8; N]>, &'static str> {
    (flags & mask != 0).then(|| field(input)).transpose()
}

fn extension(flags: u8, input: &mut &[u8]) -> Result<u8, &'static str> {
    Ok(optional::<1>(flags, 0x80, input)?.unwrap_or_default()[0])
}

impl MacBody {
    pub fn parse_blocks(mut input: &[u8]) -> Result<Vec<MacBlock>, &'static str> {
        let mut blocks = Vec::new();
        while !input.is_empty() {
            let header = byte(&mut input)?;
            let extension = extension(header, &mut input)?;
            if extension & 0x80 != 0 {
                byte(&mut input)?;
            }
            let len = ((header >> 4) & 3) | ((extension >> 2) & 0x1c);
            blocks.push(MacBlock {
                id: (header & 15) | ((extension & 3) << 4),
                header,
                extension,
                value: take(&mut input, usize::from(len))?.to_vec(),
            });
        }
        Ok(blocks)
    }

    fn read(data: &mut &[u8]) -> Result<Self, &'static str> {
        let control = byte(data)?;
        let ext = extension(control, data)?;
        if ext & 0x80 != 0 {
            byte(data)?;
        }
        let len = (control & 31) | ((ext & 1) << 5);
        let mut data = take(data, usize::from(len))?;
        let mut body = Self {
            control,
            extension: ext,
            derivation_counter: optional::<1>(control, 0x40, &mut data)?.map(|x| x[0]),
            message_counter: optional(control, 0x20, &mut data)?.map(u16::from_le_bytes),
            authentication_code: optional(control, 0x20, &mut data)?,
            ..Default::default()
        };
        if control & 0x20 != 0 {
            body.encrypted_blocks = data.to_vec();
        } else {
            body.blocks = Self::parse_blocks(data)?;
        }
        Ok(body)
    }
}

impl Llc {
    fn read(data: &mut &[u8]) -> Result<Self, &'static str> {
        let flags = byte(data)?;
        let ext = extension(flags, data)?;
        if ext & 0x80 != 0 {
            byte(data)?;
        }
        let llc = Self {
            flags,
            extension: ext,
            control: optional::<1>(flags, 1, data)?.map(|x| x[0]),
            sender: optional(flags, 2, data)?,
            receiver: optional(flags, 4, data)?,
            access_number: optional::<1>(flags, 8, data)?.map(|x| x[0]),
            run_time_delay: optional(ext, 3, data)?.map(u16::from_le_bytes),
            adapter_status: optional::<1>(ext, 8, data)?.map(|x| x[0]),
        };
        Ok(llc)
    }
}

impl LpwanHeader {
    pub fn parse(frame: &[u8]) -> Result<(Self, &[u8]), &'static str> {
        if !has_mac_crc(frame) {
            return Err("Invalid LPWAN MAC CRC");
        }
        let mut data = &frame[..frame.len() - 4];
        let header = byte(&mut data)?;
        if header & 0x10 != 0 {
            return Err("Unsupported LPWAN MAC version");
        }
        let ext = extension(header, &mut data)?;
        if ext & 0x80 != 0 {
            byte(&mut data)?;
        }
        let mut elements = Vec::new();
        if header & 0x40 != 0 {
            loop {
                let value = byte(&mut data)?;
                elements.push(value);
                if value & 0x80 == 0 {
                    break;
                }
            }
        }
        let body = (header & 0x20 != 0)
            .then(|| MacBody::read(&mut data))
            .transpose()?;
        let llc = (!data.is_empty())
            .then(|| Llc::read(&mut data))
            .transpose()?;
        Ok((
            Self {
                header,
                extension: ext,
                elements,
                body,
                llc,
                upper_layer_octets: data.len(),
            },
            data,
        ))
    }

    pub fn downlink(&self) -> bool {
        matches!(self.header & 15, 12 | 13)
    }

    pub fn check(
        &self,
        application_present: bool,
        short_tpl: bool,
        long_tpl: bool,
        report: &mut OmsReport<Vec<RuleFinding>>,
    ) {
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
            });
        };
        let kind = self.header & 15;
        check(
            "LPWAN MAC frame type",
            "Annex Q, Table Q.74",
            annex_q!(82, 85),
            matches!(kind, 0..=2 | 8 | 9 | 12 | 13),
            "The assigned MAC frame types are MSNR, MRSP, MERR, MACC, MACK, MCNR and MCMD.",
        );
        check(
            "LPWAN MAC control extension",
            "Annex Q, Tables Q.66–67",
            annex_q!(77, 448),
            self.extension == 0,
            "The MAC control extension permits only MSP1 and zero reserved bits.",
        );
        let down = self.downlink();
        if let Some(&element) = self.elements.first() {
            let valid = if down {
                self.elements.len() == 1 && element & 0x0c == 0 && (element >> 4) & 7 <= 5
            } else {
                self.elements.len() <= 5 && (element & 7 >= 2 || element & 0x20 == 0)
            };
            check(
                "LPWAN MAC access element",
                "Annex Q, Tables Q.80 and Q.89",
                if down {
                    annex_q!(101, 85)
                } else {
                    annex_q!(95, 153)
                },
                valid,
                "The first MAC element and its extensions follow the encoding for the transmission direction.",
            );
        }
        let secured = self
            .body
            .as_ref()
            .is_some_and(|b| b.message_counter.is_some());
        if kind == 9 {
            check(
                "LPWAN MAC acknowledge access",
                "Annex Q, §Q.3.3.2.2",
                annex_q!(82, 752),
                self.elements.first().is_none_or(|element| element & 7 < 2),
                "MACK cannot offer a downlink access opportunity.",
            );
        }
        if kind == 1 || kind == 2 {
            check(
                "LPWAN MAC response body",
                "Annex Q, §Q.3.3.2.4",
                annex_q!(83, 568),
                if kind == 1 {
                    secured
                } else {
                    self.body.is_none()
                },
                "MRSP requires a secured body. MERR omits the body.",
            );
        }
        if let Some(b) = &self.body {
            check(
                "LPWAN MAC body extension",
                "Annex Q, Table Q.70",
                annex_q!(79, 85),
                b.extension & !1 == 0,
                "The body extension reserves all bits except the high length bit.",
            );
            if !b.blocks.is_empty() {
                check(
                    "LPWAN MAC block headers",
                    "Annex Q, Tables Q.72–73",
                    annex_q!(80, 311),
                    b.blocks
                        .iter()
                        .all(|b| b.header & 0x40 == 0 && b.extension & 0x8c == 0),
                    "Clear MAC block headers use zero for reserved bits.",
                );
            }
            if !b.blocks.is_empty() || !b.encrypted_blocks.is_empty() {
                check(
                    "LPWAN MAC block frame type",
                    "Annex Q, §Q.3.3.4.1",
                    annex_q!(84, 370),
                    matches!(kind, 0 | 1 | 12 | 13),
                    "MAC blocks are carried by MSNR, MRSP, MCMD and MCNR.",
                );
            }
            if !b.blocks.is_empty() {
                if kind == 0 && b.blocks.iter().any(|b| matches!(b.id, 0..=7 | 0x12..=0x14)) {
                    check(
                        "LPWAN MAC block protection",
                        "Annex Q, Tables Q.90–105",
                        annex_q!(102, 85),
                        secured,
                        "These block functions require authenticated encryption, including in an unsolicited MSNR.",
                    );
                }
                check(
                    "LPWAN MAC block values",
                    "Annex Q, Tables Q.92–105",
                    annex_q!(103, 155),
                    b.blocks.iter().all(|block| block.valid_value(kind)),
                    "The block length and GET or SET operation must be valid for its identifier.",
                );
                check(
                    "LPWAN MAC block identifiers",
                    "Annex Q, Table Q.91",
                    annex_q!(102, 498),
                    b.blocks
                        .iter()
                        .all(|block| matches!(block.id, 0..=7 | 0x0f..=0x14 | 0x38..=0x3e)),
                    "MAC blocks must use an assigned or manufacturer-specific identifier.",
                );
            }
            if kind == 0 && b.derivation_counter.is_some() {
                check(
                    "LPWAN MAC derivation counter protection",
                    "Annex Q, §Q.3.3.2.3",
                    annex_q!(83, 344),
                    secured,
                    "An unsolicited uplink carrying the derivation counter requires MAC security.",
                );
            }
            if down
                && (b.derivation_counter.is_some()
                    || !b.blocks.is_empty()
                    || !b.encrypted_blocks.is_empty())
            {
                check(
                    "LPWAN MAC command security",
                    "Annex Q, §Q.3.3.3.1",
                    annex_q!(84, 120),
                    secured,
                    "MAC commands carrying blocks or a derivation counter require security.",
                );
            }
        }
        if matches!(kind, 0..=2 | 8 | 9 | 12 | 13) {
            let valid = match (kind, &self.llc) {
                (9, llc) => llc.is_none(),
                (_, Some(llc)) if down => {
                    llc.receiver.is_some() && (llc.flags & 0x40 == 0 || llc.sender.is_some())
                }
                (_, Some(llc)) => llc.sender.is_some(),
                _ => false,
            };
            check(
                "LPWAN LLC addressing",
                "Annex Q, §Q.3.3, Table Q.74",
                annex_q!(82, 85),
                valid,
                "Uplinks identify their sender and downlinks their receiver. MACK omits the LLC.",
            );
        }
        if let Some(llc) = &self.llc {
            let meter = if down { llc.receiver } else { llc.sender };
            if application_present && meter.is_some_and(|address| address[7] == 0x37) {
                check(
                    "LPWAN adapter application address",
                    "§3.1.4.4, Table 5",
                    primary!(35, 252),
                    long_tpl,
                    "A telegram through an external radio adapter supplies its application layer address in a long transport header.",
                );
            }
            check(
                "LPWAN LLC control bits",
                "Annex Q, Tables Q.109–110",
                annex_q!(123, 267),
                llc.flags & 0x20 == 0 && llc.extension & 0xf0 == 0 && llc.extension & 3 != 3,
                "Reserved LLC bits and the reserved delay resolution are prohibited.",
            );
            check(
                "LPWAN upper layer presence",
                "Annex Q, Table Q.109",
                annex_q!(123, 267),
                (llc.flags & 0x10 != 0) == application_present,
                "The upper layer flag matches the presence of the control information byte.",
            );
            if llc.adapter_status.is_some() {
                check(
                    "LPWAN radio adapter status",
                    "Annex Q, §Q.4.2.10",
                    annex_q!(124, 737),
                    !down && !short_tpl,
                    "Only an uplink from a radio adapter may provide radio adapter status in the LLC.",
                );
            }
        }
        if secured {
            report.push(RuleFinding {
                rule: "LPWAN MAC authentication",
                reference: "Annex Q, §Q.3.4.5.2",
                source_url: annex_q!(91, 153),
                status: if self.body.as_ref().is_some_and(|b| b.authenticated) {
                    RuleStatus::Passed
                } else {
                    RuleStatus::CouldNotCheck
                },
                detail: if self.body.as_ref().is_some_and(|b| b.authenticated) {
                    "The MAC authentication tag matches the secured body and link context."
                } else {
                    "MAC authentication needs the MAC key and derivation counter for this link."
                },
                record_number: None,
            });
        }
    }
}

#[derive(Default)]
pub struct LpwanHistory {
    pending: alloc::collections::BTreeMap<[u8; 8], PendingCommand>,
}

struct PendingCommand {
    gateway: Option<[u8; 8]>,
    response: bool,
    counter: Option<u16>,
    block_ids: Option<Vec<u8>>,
}

impl LpwanHistory {
    pub fn observe(
        &mut self,
        frame: &LpwanHeader,
        consecutive: bool,
        report: &mut OmsReport<Vec<RuleFinding>>,
    ) {
        if !consecutive {
            return;
        }
        let Some(llc) = &frame.llc else {
            return;
        };
        if frame.downlink() {
            if let Some(meter) = llc.receiver {
                let gateway = llc.sender.filter(|_| llc.flags & 0x40 != 0);
                let body = frame.body.as_ref();
                self.pending.insert(
                    meter,
                    PendingCommand {
                        gateway,
                        response: frame.header & 15 == 13,
                        counter: body.and_then(|body| body.message_counter),
                        block_ids: body
                            .filter(|body| body.authenticated)
                            .map(|body| body.blocks.iter().map(|block| block.id).collect()),
                    },
                );
            }
        } else if let Some(pending) = llc.sender.and_then(|sender| self.pending.remove(&sender)) {
            if let Some(gateway) = pending.gateway {
                report.push(RuleFinding {
                    rule: "LPWAN requested receiver address",
                    reference: "Annex Q, §Q.4.2.2",
                    source_url: annex_q!(124, 188),
                    status: if llc.receiver == Some(gateway) {
                        RuleStatus::Passed
                    } else {
                        RuleStatus::Failed
                    },
                    detail: "The following uplink must address the gateway that set RRX.",
                    record_number: None,
                });
            }
            if pending.response
                && frame.header & 15 == 1
                && let Some(body) = frame.body.as_ref()
            {
                if let Some(counter) = pending.counter {
                    report.push(RuleFinding {
                        rule: "LPWAN MAC response counter",
                        reference: "Annex Q, §Q.3.4.3.2.3",
                        source_url: annex_q!(89, 371),
                        status: if Some(counter) == body.message_counter {
                            RuleStatus::Passed
                        } else {
                            RuleStatus::Failed
                        },
                        detail: "A secured response uses the message counter of the command.",
                        record_number: None,
                    });
                }
                if let Some(mut expected) = pending.block_ids
                    && body.authenticated
                {
                    let mut ids: Vec<_> = body.blocks.iter().map(|block| block.id).collect();
                    expected.sort_unstable();
                    ids.sort_unstable();
                    report.push(RuleFinding { rule: "LPWAN MAC response blocks", reference: "Annex Q, §Q.3.3.4.3", source_url: annex_q!(84, 672), status: if ids == expected { RuleStatus::Passed } else { RuleStatus::Failed }, detail: "The response returns the identifiers of all blocks in the authenticated command.", record_number: None });
                }
            }
            if pending.response {
                report.push(RuleFinding {
                    rule: "LPWAN MAC command response",
                    reference: "Annex Q, §Q.3.3.2.4",
                    source_url: annex_q!(83, 568),
                    status: if matches!(frame.header & 15, 1 | 2) {
                        RuleStatus::Passed
                    } else {
                        RuleStatus::Failed
                    },
                    detail: "An MCMD requires an MRSP or MERR from the addressed end-device.",
                    record_number: None,
                });
            }
        }
    }
}
