use crate::{
    compact::{CompactFrame, FormatFrame},
    data_information::{DataFieldCoding, DataType},
    ApplicationLayerError, ControlInformation, LongTplHeader, ShortTplHeader,
};

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub enum TransportHeader {
    None,
    Short(ShortTplHeader),
    Long(LongTplHeader),
}

impl TransportHeader {
    pub fn short_header(&self) -> Option<&ShortTplHeader> {
        match self {
            Self::None => None,
            Self::Short(header) => Some(header),
            Self::Long(header) => Some(&header.short_tpl_header),
        }
    }

    pub fn long_header(&self) -> Option<&LongTplHeader> {
        match self {
            Self::Long(header) => Some(header),
            _ => None,
        }
    }

    pub fn is_encrypted(&self) -> bool {
        self.short_header()
            .is_some_and(ShortTplHeader::is_encrypted)
    }

    pub const fn size(&self) -> usize {
        match self {
            Self::None => 0,
            Self::Short(_) => 4,
            Self::Long(_) => 12,
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApplicationProtocol {
    Synchronize,
    RequestSelectedApplication,
    SetBaudRate(u32),
    ReadRam,
    WriteRam,
    StartCalibration,
    ReadEeprom,
    StartSoftwareTest,
    LegacyHashProcedure(u8),
    ApplicationError,
    Alarm,
    ClockSet,
    ClockAdjust,
    Format,
    Compact,
    DlmsCosem,
    Sml,
    TransportOnly,
    RequestCompactFrame,
    RequestFormatFrame,
    NetworkLayer,
    NetworkManagement,
}

impl ApplicationProtocol {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Synchronize => "synchronize",
            Self::RequestSelectedApplication => "request_selected_application",
            Self::SetBaudRate(_) => "set_baud_rate",
            Self::ReadRam => "read_ram",
            Self::WriteRam => "write_ram",
            Self::StartCalibration => "start_calibration",
            Self::ReadEeprom => "read_eeprom",
            Self::StartSoftwareTest => "start_software_test",
            Self::LegacyHashProcedure(_) => "legacy_hash_procedure",
            Self::ApplicationError => "application_error",
            Self::Alarm => "alarm",
            Self::ClockSet => "clock_set",
            Self::ClockAdjust => "clock_adjust",
            Self::Format => "format",
            Self::Compact => "compact",
            Self::DlmsCosem => "dlms_cosem",
            Self::Sml => "sml",
            Self::TransportOnly => "transport_only",
            Self::RequestCompactFrame => "request_compact_frame",
            Self::RequestFormatFrame => "request_format_frame",
            Self::NetworkLayer => "network_layer",
            Self::NetworkManagement => "network_management",
        }
    }

    pub const fn has_external_payload(self) -> bool {
        matches!(
            self,
            Self::ReadRam
                | Self::WriteRam
                | Self::StartCalibration
                | Self::ReadEeprom
                | Self::StartSoftwareTest
                | Self::LegacyHashProcedure(_)
                | Self::DlmsCosem
                | Self::Sml
                | Self::NetworkLayer
                | Self::NetworkManagement
        )
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub struct ApplicationMessage<'a> {
    pub control_information: u8,
    pub protocol: ApplicationProtocol,
    pub transport_header: TransportHeader,
    pub extended_link_layer: Option<crate::extended_link_layer::ExtendedLinkLayer>,
    pub payload: &'a [u8],
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, PartialEq)]
pub enum ApplicationPayload<'a> {
    Command,
    BaudRate(u32),
    Error {
        code: u8,
        details: &'a [u8],
    },
    Alarm {
        status: Option<u8>,
        details: &'a [u8],
    },
    Clock {
        operation: ClockOperation,
        value: DataType<'a>,
    },
    Format(FormatFrame<'a>),
    Compact(CompactFrame<'a>),
    External(&'a [u8]),
}

#[cfg_attr(feature = "serde", derive(serde::Serialize))]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockOperation {
    Set,
    Add,
    Subtract,
}

impl<'a> ApplicationMessage<'a> {
    pub(crate) fn parse(
        control: ControlInformation,
        data: &'a [u8],
    ) -> Result<Self, ApplicationLayerError> {
        use ApplicationProtocol as P;
        use ControlInformation as C;
        let (protocol, header_size) = match control {
            C::SynchronizeSlave => (P::Synchronize, 0),
            C::RequestSelectedApplication => (P::RequestSelectedApplication, 0),
            C::SetBaudRate300 => (P::SetBaudRate(300), 0),
            C::SetBaudRate600 => (P::SetBaudRate(600), 0),
            C::SetBaudRate1200 => (P::SetBaudRate(1200), 0),
            C::SetBaudRate2400 => (P::SetBaudRate(2400), 0),
            C::SetBaudRate4800 => (P::SetBaudRate(4800), 0),
            C::SetBaudRate9600 => (P::SetBaudRate(9600), 0),
            C::SetBaudRate19200 => (P::SetBaudRate(19200), 0),
            C::SetBaudRate38400 => (P::SetBaudRate(38400), 0),
            C::OutputRAMContent => (P::ReadRam, 0),
            C::WriteRAMContent => (P::WriteRam, 0),
            C::StartCalibrationTestMode => (P::StartCalibration, 0),
            C::ReadEEPROM => (P::ReadEeprom, 0),
            C::StartSoftwareTest => (P::StartSoftwareTest, 0),
            C::HashProcedure(procedure) => (P::LegacyHashProcedure(procedure), 0),
            C::SendErrorStatus => (P::ApplicationError, 0),
            C::ApplicationErrorShortTransport => (P::ApplicationError, 4),
            C::ApplicationErrorLongTransport => (P::ApplicationError, 12),
            C::SendAlarmStatus => (P::Alarm, 0),
            C::AlarmShortTransport => (P::Alarm, 4),
            C::AlarmLongTransport => (P::Alarm, 12),
            C::ClockSyncAbsolute => (P::ClockSet, 12),
            C::ClockSyncRelative => (P::ClockAdjust, 12),
            C::ApplicationLayerFormatFrameNoTransport => (P::Format, 0),
            C::ApplicationLayerFormatFrameShortTransport => (P::Format, 4),
            C::ApplicationLayerFormatFrameLongTransport => (P::Format, 12),
            C::ApplicationLayerCompactFrameNoTransport => (P::Compact, 0),
            C::ApplicationLayerCompactFrameShortTransport => (P::Compact, 4),
            C::CosemDataWithLongTransportLayer | C::CosemApplicationLayerLongTransport => {
                (P::DlmsCosem, 12)
            }
            C::CosemDataWithShortTransportLayer | C::CosemApplicationLayerShortTransport => {
                (P::DlmsCosem, 4)
            }
            C::ObisDataReservedLongTransportLayer
            | C::ObisApplicationLayerReservedLongTransport => (P::Sml, 12),
            C::ObisDataReservedShortTransportLayer
            | C::ObisApplicationLayerReservedShortTransport => (P::Sml, 4),
            C::TransportLayerLongReadoutToMeter | C::TransportLayerLongMeterToReadout => {
                (P::TransportOnly, 12)
            }
            C::TransportLayerShortMeterToReadout => (P::TransportOnly, 4),
            C::TransportLayerCompactFrame => (P::RequestCompactFrame, 12),
            C::TransportLayerFormatFrame => (P::RequestFormatFrame, 12),
            C::NetworkLayerData => (P::NetworkLayer, 0),
            C::FutureUse => (P::NetworkManagement, 12),
            C::NetworkManagementApplication | C::NetworkManagementDataReserved => {
                (P::NetworkManagement, 0)
            }
            _ => {
                return Err(ApplicationLayerError::InvalidControlInformation {
                    byte: data.first().copied().unwrap_or(0),
                })
            }
        };
        let header_bytes = data
            .get(1..)
            .ok_or(ApplicationLayerError::InsufficientData)?;
        let transport_header = match header_size {
            4 => TransportHeader::Short(ShortTplHeader::parse(header_bytes)?),
            12 => TransportHeader::Long(LongTplHeader::parse(header_bytes, false)?),
            _ => TransportHeader::None,
        };
        let message = Self {
            control_information: *data
                .first()
                .ok_or(ApplicationLayerError::MissingControlInformation)?,
            protocol,
            transport_header,
            extended_link_layer: None,
            payload: data
                .get(1 + header_size..)
                .ok_or(ApplicationLayerError::InsufficientData)?,
        };
        if !message.transport_header.is_encrypted() {
            message.decode()?;
        }
        Ok(message)
    }

    pub fn decode(&self) -> Result<ApplicationPayload<'a>, ApplicationLayerError> {
        if self.transport_header.is_encrypted() {
            return Err(ApplicationLayerError::EncryptedPayload);
        }
        self.decode_payload(self.payload)
    }

    pub fn decode_payload<'b>(
        &self,
        payload: &'b [u8],
    ) -> Result<ApplicationPayload<'b>, ApplicationLayerError> {
        use ApplicationPayload as V;
        use ApplicationProtocol as P;
        let payload = if self.transport_header.is_encrypted() {
            payload
                .strip_prefix(&[0x2F, 0x2F])
                .ok_or(ApplicationLayerError::InvalidPayload {
                    field: "decryption verification",
                })?
        } else {
            payload
        };
        match self.protocol {
            P::Synchronize
            | P::RequestSelectedApplication
            | P::TransportOnly
            | P::RequestCompactFrame
            | P::RequestFormatFrame
            | P::SetBaudRate(_) => {
                require_length(payload, 0)?;
                Ok(match self.protocol {
                    P::SetBaudRate(baud) => V::BaudRate(baud),
                    _ => V::Command,
                })
            }
            P::ApplicationError => Ok(V::Error {
                code: payload.first().copied().unwrap_or(0),
                details: payload.get(1..).unwrap_or_default(),
            }),
            P::Alarm => Ok(V::Alarm {
                status: payload.first().copied(),
                details: payload.get(1..).unwrap_or_default(),
            }),
            P::ClockSet | P::ClockAdjust => parse_clock(self.protocol, payload),
            P::Format => FormatFrame::parse(payload).map(V::Format),
            P::Compact => CompactFrame::parse(payload).map(V::Compact),
            _ => Ok(V::External(payload)),
        }
    }
}

pub(crate) fn require_length(data: &[u8], expected: usize) -> Result<(), ApplicationLayerError> {
    if data.len() == expected {
        Ok(())
    } else {
        Err(ApplicationLayerError::InvalidDataLength {
            expected,
            actual: data.len(),
        })
    }
}

fn parse_clock(
    protocol: ApplicationProtocol,
    payload: &[u8],
) -> Result<ApplicationPayload<'_>, ApplicationLayerError> {
    let payload = payload.strip_prefix(&[0x2F, 0x2F]).unwrap_or(payload);
    require_length(payload, 14)?;
    let control = *payload
        .first()
        .ok_or(ApplicationLayerError::InsufficientData)?;
    let operation = match (protocol, control) {
        (ApplicationProtocol::ClockSet, 0) => ClockOperation::Set,
        (ApplicationProtocol::ClockAdjust, 1) => ClockOperation::Add,
        (ApplicationProtocol::ClockAdjust, 2) => ClockOperation::Subtract,
        _ => {
            return Err(ApplicationLayerError::InvalidPayload {
                field: "clock control",
            })
        }
    };
    let value_size = if operation == ClockOperation::Set {
        6
    } else {
        3
    };
    let reserved = payload
        .get(1 + value_size..10)
        .ok_or(ApplicationLayerError::InsufficientData)?;
    if reserved.iter().any(|byte| *byte != 0) || payload.get(10..) != Some(&[0x2F; 4]) {
        return Err(ApplicationLayerError::InvalidPayload {
            field: "clock verification",
        });
    }
    let coding = if operation == ClockOperation::Set {
        DataFieldCoding::DateTimeTypeI
    } else {
        DataFieldCoding::DateTimeTypeJ
    };
    let value = coding
        .parse(
            payload
                .get(1..1 + value_size)
                .ok_or(ApplicationLayerError::InsufficientData)?,
            None,
        )
        .map_err(|_| ApplicationLayerError::InvalidPayload {
            field: "clock time",
        })?
        .value
        .ok_or(ApplicationLayerError::InvalidPayload {
            field: "clock time",
        })?;
    Ok(ApplicationPayload::Clock { operation, value })
}
