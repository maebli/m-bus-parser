use crate::telegram::{RuleFinding, RuleStatus, TelegramReport};

#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ImageTransfer {
    pub segment_id: Option<u8>,
    pub segment_length: Option<u16>,
    pub function: u8,
    pub subfunction: Option<u8>,
    pub image_identifier: Option<[u8; 8]>,
    pub total_blocks: Option<u16>,
    pub block_size: Option<u16>,
    pub repetitions: Option<u8>,
    pub block_number: Option<u16>,
    pub remaining: Option<u16>,
    pub data_length: Option<usize>,
    pub lost_total: Option<u16>,
    pub lost_list_count: Option<usize>,
    pub prepare_result: Option<u8>,
    pub mac_algorithm: Option<u8>,
    pub mac_key_id: Option<u8>,
    pub mac_key_version: Option<u8>,
    pub mac: Option<[u8; 16]>,
    pub firmware_cmac_matches: Option<bool>,
    pub captured_image: Option<CapturedImage>,
    pub structure_complete: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(
    feature = "serde",
    derive(serde::Serialize, serde::Deserialize),
    serde(rename_all = "snake_case")
)]
pub enum CapturedImage {
    Verified,
    Mismatch,
    Missing,
    Conflicting,
    Dimensions,
    UnknownMeter,
}

impl CapturedImage {
    fn matches(self) -> Option<bool> {
        match self {
            Self::Verified => Some(true),
            Self::Mismatch => Some(false),
            _ => None,
        }
    }
    fn detail(self) -> &'static str {
        match self {
            Self::Verified | Self::Mismatch => {
                "The CMAC covers the captured image blocks in numeric index order."
            }
            Self::Missing => {
                "The capture lacks at least one block or a complete prepare declaration."
            }
            Self::Conflicting => {
                "Conflicting prepare declarations or duplicate block contents prevent authentication of a unique image."
            }
            Self::Dimensions => {
                "The captured block numbers or lengths disagree with the prepare declaration."
            }
            Self::UnknownMeter => {
                "A meter address is needed to join the prepare and captured transfer blocks."
            }
        }
    }
}

fn take<'a, const N: usize>(input: &mut &'a [u8]) -> Option<&'a [u8; N]> {
    let (field, next) = input.split_first_chunk::<N>()?;
    *input = next;
    Some(field)
}

impl ImageTransfer {
    pub fn parse(value: &[u8]) -> Option<Self> {
        let (function, body) = value.split_first()?;
        let mut frame = Self {
            function: *function,
            ..Default::default()
        };
        if let Some((flags, fields)) = body.split_first() {
            frame.subfunction = Some(*flags);
            frame.structure_complete = frame.read_fields(*flags, fields).is_some();
        }
        Some(frame)
    }

    fn identifier(&mut self, present: bool, data: &mut &[u8]) -> Option<()> {
        if present {
            self.image_identifier = Some(*take(data)?);
        }
        Some(())
    }

    fn sizes(&mut self, present: bool, data: &mut &[u8]) -> Option<()> {
        if present {
            self.total_blocks = Some(u16::from_le_bytes(*take(data)?));
            self.block_size = Some(u16::from_le_bytes(*take(data)?));
            self.repetitions = Some(take::<1>(data)?[0]);
        }
        Some(())
    }

    fn read_fields(&mut self, flags: u8, mut data: &[u8]) -> Option<()> {
        match self.function {
            0 => {
                if flags & 0x10 != 0 {
                    take::<5>(&mut data)?;
                }
                self.sizes(flags & 4 != 0, &mut data)?;
                self.identifier(flags & 8 != 0, &mut data)?;
                if flags & 1 != 0 {
                    let [algorithm, key, version] = *take(&mut data)?;
                    self.mac_algorithm = Some(algorithm);
                    self.mac_key_id = Some(key);
                    self.mac_key_version = Some(version);
                    self.mac = Some(*take(&mut data)?);
                }
                if flags & 0x20 != 0 {
                    take::<2>(&mut data)?;
                }
            }
            0x80 => {
                self.prepare_result = Some(take::<1>(&mut data)?[0]);
                self.identifier(flags & 8 != 0, &mut data)?;
                self.sizes(flags & 2 != 0, &mut data)?;
                if flags & 1 != 0 {
                    take::<2>(&mut data)?;
                }
            }
            1 => {
                self.identifier(flags & 2 != 0, &mut data)?;
                take::<2>(&mut data)?;
            }
            2 => {
                self.identifier(flags & 8 != 0, &mut data)?;
                self.block_number = Some(u16::from_le_bytes(*take(&mut data)?));
                if flags & 0x10 != 0 {
                    self.remaining = Some(u16::from_le_bytes(*take(&mut data)?));
                }
                if flags & 1 != 0 {
                    take::<2>(&mut data)?;
                }
                self.data_length = Some(data.len());
                return (!data.is_empty()).then_some(());
            }
            3..=5 => {
                self.identifier(flags & 2 != 0, &mut data)?;
                if flags & 1 != 0 {
                    take::<2>(&mut data)?;
                }
            }
            6 => {
                self.identifier(flags & 2 != 0, &mut data)?;
                take::<4>(&mut data)?;
            }
            7 => self.identifier(flags & 4 != 0, &mut data)?,
            0x82 | 0x84..=0x87 => self.identifier(flags & 0x10 != 0, &mut data)?,
            0x83 => {
                self.identifier(flags & 8 != 0, &mut data)?;
                if flags & 4 != 0 {
                    self.lost_total = Some(u16::from_le_bytes(*take(&mut data)?));
                }
                if flags & 2 != 0 {
                    self.lost_list_count = Some(data.len() / 2);
                    return (!data.is_empty() && data.len().is_multiple_of(2)).then_some(());
                }
            }
            _ => return None,
        }
        data.is_empty().then_some(())
    }

    pub fn assess(
        &self,
        wrapped: bool,
        wrapper: Option<(u8, u8, u8, bool)>,
        command: bool,
        report: &mut TelegramReport,
    ) {
        if matches!(self.function, 1 | 8 | 0x88 | 0x70..=0x7f | 0xf0..=0xff) {
            report.push(RuleFinding {rule:"OMS firmware transfer function", reference:"Annex M, section M.2.6.1",source_url:annex_m!(26,150),status:RuleStatus::NotApplicable,detail:"UC-05 specifies the unicast transfer procedure. This function belongs to another ITP operation.",record_number:None});
            return;
        }
        let source = match self.function & 0x7f {
            0 => ("Annex M, Table M.17", annex_m!(29, 161)),
            2 => ("Annex M, Table M.18", annex_m!(30, 205)),
            3 => ("Annex M, Table M.19", annex_m!(31, 77)),
            4 => ("Annex M, Table M.16", annex_m!(28, 210)),
            5 => ("Annex M, Table M.20", annex_m!(32, 161)),
            6 => ("Annex M, Table M.21", annex_m!(33, 178)),
            _ => ("Annex M, Table M.22", annex_m!(34, 144)),
        };
        let mut check = |rule, valid: Option<bool>, detail| {
            report.push(RuleFinding {
                rule,
                reference: source.0,
                source_url: source.1,
                detail,
                record_number: None,
                status: match valid {
                    Some(true) => RuleStatus::Passed,
                    Some(false) => RuleStatus::Failed,
                    None => RuleStatus::CouldNotCheck,
                },
            })
        };
        let function = self.function;
        let oms = matches!(function,0|2..=7|0x80|0x82..=0x87);
        check(
            "OMS firmware transfer function",
            Some(oms),
            "UC-05 permits the prepare, transfer, completion, state, validation, activation and termination functions.",
        );
        if !oms {
            return;
        }
        check(
            "Firmware transfer direction",
            Some((function < 0x80) == command),
            "The function must be a command or a response in its CI direction.",
        );
        check(
            "Firmware transfer fields",
            Some(self.structure_complete),
            "The APDU must include the fields selected by its subfunction with their defined lengths.",
        );
        let allowed = match function {
            0 | 0x80 => 0x3f,
            2 | 0x82 | 0x84..=0x87 => 0x1f,
            0x83 => 0x0f,
            7 => 7,
            _ => 3,
        };
        check(
            "Firmware reserved subfunction bits",
            self.subfunction.map(|f| f & !allowed == 0),
            "Reserved subfunction bits must be zero.",
        );
        check(
            "Firmware image identifier",
            Some(self.image_identifier.is_some()),
            "The UC-05 function requires an eight-byte image identifier.",
        );
        if function == 0 {
            check(
                "Firmware prepare multicast fields",
                self.subfunction.map(|f| f & 0x10 == 0),
                "The OMS prepare command must omit multicast date, time and pace.",
            );
            check(
                "Firmware transfer size declaration",
                Some(
                    self.total_blocks.is_some_and(|n| n > 0)
                        && self.block_size.is_some_and(|n| n > 0)
                        && self.repetitions == Some(0),
                ),
                "Unicast preparation requires a positive block count and size and zero multicast repetitions.",
            );
            check(
                "Firmware image authentication fields",
                Some(
                    self.mac_algorithm == Some(4)
                        && self.mac_key_id == Some(0x18)
                        && self.mac.is_some(),
                ),
                "The prepare command requires a full AES-128 CMAC and firmware key ID 18.",
            );
            check(
                "Firmware image CMAC",
                self.firmware_cmac_matches
                    .or_else(|| self.captured_image.and_then(CapturedImage::matches)),
                if self.firmware_cmac_matches.is_some() {
                    "The declared 16-byte CMAC must authenticate the supplied firmware image."
                } else {
                    self.captured_image.map(CapturedImage::detail).unwrap_or("The declared CMAC requires the firmware key and a complete image or a capture containing its blocks.")
                },
            );
        }
        if function == 0x80 {
            check(
                "Firmware prepare result",
                self.prepare_result.map(|v| v < 8),
                "The preparation result code must be assigned.",
            );
        }
        if function == 0x80 && self.subfunction.is_some_and(|bits| bits & 2 != 0) {
            check(
                "Firmware supported block size",
                self.block_size.map(|size| size >= 190),
                "A prepare response that declares the supported block size must allow at least 190 bytes.",
            );
        }
        if function == 0x83 {
            check(
                "Firmware lost block list",
                Some(
                    self.lost_total
                        .is_some_and(|total| match self.lost_list_count {
                            Some(entries) => entries > 0 && entries <= usize::from(total),
                            None => (total == 0) == self.subfunction.is_some_and(|f| f & 1 == 0),
                        }),
                ),
                "The completion response requires the total lost count. A reported list must fit the total and omit entries when no blocks were lost.",
            );
        }
        if matches!(function, 0 | 0x80) {
            check(
                "Firmware prepare application protection",
                Some(wrapped),
                "UC-05 requires an ASP10 SITP envelope for the prepare exchange.",
            );
        }
        if wrapped && matches!(function, 0 | 6 | 7 | 0x80 | 0x86 | 0x87) {
            check(
                "Firmware command application protection",
                wrapper.map(|(bcf, key, dsi, _)| {
                    bcf == if command { 0x20 } else { 0xa0 } && key == 0x23 && dsi == 0x32
                }),
                "The protected firmware operation uses ASP10 with PID 06 and application key 23.",
            );
            check(
                "Firmware target time",
                wrapper.map(|(_, _, _, timing)| timing),
                "The ASP10 target time specifies immediate action for prepare and terminate, and delegates the activation time to ITP for activate.",
            );
        }
    }
}

impl ImageTransfer {
    pub fn check_block_dimensions(
        &self,
        prepared: Option<(u16, u16)>,
        report: &mut TelegramReport,
    ) {
        let mut check = |rule, valid: Option<bool>, detail| {
            report.push(RuleFinding {
                rule,
                reference: "Annex M, Table M.18",
                source_url: annex_m!(30, 205),
                status: match valid {
                    Some(true) => RuleStatus::Passed,
                    Some(false) => RuleStatus::Failed,
                    None => RuleStatus::CouldNotCheck,
                },
                detail,
                record_number: None,
            });
        };
        let count = prepared.map(|(count, _)| count);
        check(
            "Firmware transfer block number",
            self.block_number
                .zip(count)
                .map(|(block, total)| block < total),
            "The block number must be within the range declared by a preceding prepare for this meter and image.",
        );
        check(
            "Firmware transfer block size",
            self.data_length
                .zip(prepared)
                .map(|(bytes, (_, size))| bytes == usize::from(size)),
            "Each transmitted image block must have the declared number of bytes, including image-provider padding.",
        );
        if let Some(remaining) = self.remaining {
            check(
                "Firmware remaining block count",
                count.map(|total| remaining < total),
                "The remaining count must be less than the declared number of blocks.",
            );
        }
    }
}

pub fn check_segments(complete: bool, report: &mut TelegramReport) {
    report.push(RuleFinding{rule:"Firmware transfer segments",reference:"Annex M, section M.2.6; EN 13757-3 I.2.2",source_url:annex_m!(26,150),status:if complete {RuleStatus::Passed} else {RuleStatus::Failed},detail:"The APDU must contain complete length-prefixed segments with a segment ID and function.",record_number:None});
}

pub fn check_reply(matches: Option<bool>, report: &mut TelegramReport) {
    report.push(RuleFinding{rule:"Firmware response segment",reference:"Annex M, section M.2.6; EN 13757-3 I.2.2",source_url:annex_m!(26,114),status:match matches {Some(true)=>RuleStatus::Passed,Some(false)=>RuleStatus::Failed,None=>RuleStatus::CouldNotCheck},detail:"Each response segment must identify the corresponding command by segment ID, function and image.",record_number:None});
}
