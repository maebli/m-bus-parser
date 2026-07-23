//! Legacy rSCADA/libmbus normalized XML output.
//!
//! Reproduces the output of libmbus's `mbus_frame_data_xml_normalized()`
//! (https://github.com/rscada/libmbus) byte for byte, so the parser can be
//! diffed against the reference `.norm.xml` files in `tests/rscada`.

use crate::mbus_data::MbusData;
use crate::user_data;
use wired_mbus_link_layer as frames;

const XML_PROCESSING_INSTRUCTION: &str = "<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?>\n";

/// Render a raw frame in the legacy libmbus normalized XML format.
pub(crate) fn render_from_bytes(data: &[u8]) -> String {
    let Ok(parsed) = MbusData::<frames::WiredFrame>::try_from(data) else {
        return "Error: Could not parse data as a wired M-Bus frame".to_string();
    };

    match &parsed.user_data {
        Some(user_data::UserDataBlock::VariableDataStructureWithLongTplHeader {
            long_tpl_header,
            ..
        }) => render_variable(long_tpl_header, parsed.data_records.as_ref()),
        Some(user_data::UserDataBlock::FixedDataStructure {
            identification_number,
            access_number,
            status,
            device_type_and_unit,
            counter1,
            counter2,
        }) => render_fixed(
            identification_number.number,
            *access_number,
            status.bits(),
            *device_type_and_unit,
            counter1,
            counter2,
        ),
        _ => "Error: Unsupported frame type for legacy XML output".to_string(),
    }
}

fn render_variable(
    header: &user_data::LongTplHeader,
    records: Option<&user_data::DataRecords>,
) -> String {
    let mut out = String::new();
    out.push_str(XML_PROCESSING_INSTRUCTION);
    out.push_str("<MBusData>\n\n");
    out.push_str(&render_variable_header(header));

    if let Some(records) = records {
        for (i, record) in records.clone().flatten().enumerate() {
            out.push_str(&render_variable_record(i, &record));
        }
    }

    out.push_str("</MBusData>\n");
    out
}

fn render_variable_header(header: &user_data::LongTplHeader) -> String {
    let manufacturer = match &header.manufacturer {
        Ok(code) => code.code.iter().collect::<String>(),
        Err(_) => String::new(),
    };
    let medium = u8::from(header.device_type);
    let mut out = String::new();
    out.push_str("    <SlaveInformation>\n");
    // libmbus prints the raw BCD digits via %llX which drops leading zeros.
    out.push_str(&format!(
        "        <Id>{}</Id>\n",
        header.identification_number.number
    ));
    out.push_str(&format!(
        "        <Manufacturer>{}</Manufacturer>\n",
        manufacturer
    ));
    out.push_str(&format!("        <Version>{}</Version>\n", header.version));
    out.push_str(&format!(
        "        <ProductName>{}</ProductName>\n",
        xml_encode(&product_name(
            &manufacturer,
            header.version,
            medium,
            header.identification_number.number,
        ))
    ));
    out.push_str(&format!(
        "        <Medium>{}</Medium>\n",
        xml_encode(&medium_lookup(medium))
    ));
    out.push_str(&format!(
        "        <AccessNumber>{}</AccessNumber>\n",
        header.short_tpl_header.access_number
    ));
    out.push_str(&format!(
        "        <Status>{:02X}</Status>\n",
        header.short_tpl_header.status.bits()
    ));
    let signature = header.short_tpl_header.configuration_field.raw();
    out.push_str(&format!(
        "        <Signature>{:02X}{:02X}</Signature>\n",
        (signature >> 8) as u8,
        (signature & 0xFF) as u8
    ));
    out.push_str("    </SlaveInformation>\n\n");
    out
}

enum DecodedValue {
    Real(f64),
    Str(String),
}

struct NormalizedRecord {
    function: String,
    storage_number: u64,
    tariff: Option<u64>,
    device: u64,
    unit: String,
    quantity: String,
    value: DecodedValue,
}

fn render_variable_record(index: usize, record: &user_data::data_record::DataRecord) -> String {
    let mut out = format!("    <DataRecord id=\"{}\">\n", index);
    if let Some(normalized) = normalize_record(record) {
        out.push_str(&format!(
            "        <Function>{}</Function>\n",
            xml_encode(&normalized.function)
        ));
        out.push_str(&format!(
            "        <StorageNumber>{}</StorageNumber>\n",
            normalized.storage_number
        ));
        if let Some(tariff) = normalized.tariff {
            out.push_str(&format!("        <Tariff>{}</Tariff>\n", tariff));
            out.push_str(&format!("        <Device>{}</Device>\n", normalized.device));
        }
        out.push_str(&format!(
            "        <Unit>{}</Unit>\n",
            xml_encode(&normalized.unit)
        ));
        out.push_str(&format!(
            "        <Quantity>{}</Quantity>\n",
            xml_encode(&normalized.quantity)
        ));
        match normalized.value {
            DecodedValue::Real(value) => {
                out.push_str(&format!("        <Value>{:.6}</Value>\n", value));
            }
            DecodedValue::Str(value) => {
                out.push_str(&format!("        <Value>{}</Value>\n", xml_encode(&value)));
            }
        }
    }
    out.push_str("    </DataRecord>\n\n");
    out
}

fn normalize_record(record: &user_data::data_record::DataRecord) -> Option<NormalizedRecord> {
    let dib = &record
        .data_record_header
        .raw_data_record_header
        .data_information_block;
    let dif = dib.data_information_field.data;
    let difes: Vec<u8> = dib
        .data_information_field_extension
        .clone()
        .map(|extensions| extensions.map(|dife| dife.data).collect())
        .unwrap_or_default();

    let header_size = record.data_record_header.get_size();
    let data = record.raw_bytes.get(header_size..).unwrap_or(&[]);

    let storage_number = storage_number(dif, &difes);
    let tariff = if difes.is_empty() {
        None
    } else {
        Some(tariff(&difes))
    };
    let device = device(&difes);

    if dif == 0x0F || dif == 0x1F {
        // Manufacturer specific / more records follow: opaque tail, no VIB.
        return Some(NormalizedRecord {
            function: if dif == 0x1F {
                "More records follow".to_string()
            } else {
                "Manufacturer specific".to_string()
            },
            storage_number,
            tariff,
            device,
            unit: String::new(),
            quantity: String::new(),
            value: DecodedValue::Str(bin_decode(data)),
        });
    }

    let vib = record
        .data_record_header
        .raw_data_record_header
        .value_information_block
        .as_ref()?;
    let vif = vib.value_information.data;
    let vifes: Vec<u8> = vib
        .value_information_extension
        .as_ref()
        .map(|extensions| extensions.iter().map(|vife| vife.data).collect())
        .unwrap_or_default();
    // libmbus stores the plaintext VIF reversed (mbus_data_str_decode).
    let custom_vif: Option<String> = vib
        .plaintext_vife
        .as_ref()
        .map(|chars| chars.iter().rev().collect());

    let value = variable_value_decode(dif, vif, vifes.first().copied(), data).ok()?;

    let (unit, quantity, normalized_value) = match value {
        DecodedValue::Real(raw) => {
            let (unit, scaled, quantity) = vib_unit_normalize(vif, &vifes, &custom_vif, raw)?;
            (unit, quantity, DecodedValue::Real(scaled))
        }
        DecodedValue::Str(text) => {
            let (unit, _, quantity) = vib_unit_normalize(vif, &vifes, &custom_vif, 0.0)?;
            (unit, quantity, DecodedValue::Str(text))
        }
    };

    Some(NormalizedRecord {
        function: function_string(dif).to_string(),
        storage_number,
        tariff,
        device,
        unit,
        quantity,
        value: normalized_value,
    })
}

fn function_string(dif: u8) -> &'static str {
    match dif & 0x30 {
        0x00 => "Instantaneous value",
        0x10 => "Maximum value",
        0x20 => "Minimum value",
        0x30 => "Value during error state",
        _ => "unknown",
    }
}

fn storage_number(dif: u8, difes: &[u8]) -> u64 {
    let mut result = u64::from((dif & 0x40) >> 6);
    let mut bit_index = 1;
    for &dife in difes {
        result |= u64::from(dife & 0x0F) << bit_index;
        bit_index += 4;
    }
    result
}

fn tariff(difes: &[u8]) -> u64 {
    let mut result = 0;
    let mut bit_index = 0;
    for &dife in difes {
        result |= u64::from((dife & 0x30) >> 4) << bit_index;
        bit_index += 2;
    }
    result
}

fn device(difes: &[u8]) -> u64 {
    let mut result = 0;
    for (bit_index, &dife) in difes.iter().enumerate() {
        result |= u64::from((dife & 0x40) >> 6) << bit_index;
    }
    result
}

fn variable_value_decode(
    dif: u8,
    vif_raw: u8,
    vife0: Option<u8>,
    data: &[u8],
) -> Result<DecodedValue, ()> {
    let vif = vif_raw & 0x7F;
    let vife = vife0.map(|v| v & 0x7F);
    let is_datetime = vif == 0x6D
        || (vif_raw == 0xFD && vife == Some(0x30))
        || (vif_raw == 0xFD && vife == Some(0x70));

    match dif & 0x0F {
        0x00 => Ok(DecodedValue::Str(String::new())),
        0x01 => Ok(DecodedValue::Real(int_decode(data.get(..1).ok_or(())?))),
        0x02 => {
            let bytes = data.get(..2).ok_or(())?;
            if vif == 0x6C {
                Ok(DecodedValue::Str(date_g_decode(
                    bytes.try_into().map_err(|_| ())?,
                )))
            } else {
                Ok(DecodedValue::Real(int_decode(bytes)))
            }
        }
        0x03 => Ok(DecodedValue::Real(int_decode(data.get(..3).ok_or(())?))),
        0x04 => {
            let bytes = data.get(..4).ok_or(())?;
            if is_datetime {
                Ok(DecodedValue::Str(date_f_decode(
                    bytes.try_into().map_err(|_| ())?,
                )))
            } else {
                Ok(DecodedValue::Real(int_decode(bytes)))
            }
        }
        0x05 => {
            let bytes: [u8; 4] = data.get(..4).ok_or(())?.try_into().map_err(|_| ())?;
            Ok(DecodedValue::Real(f64::from(f32::from_le_bytes(bytes))))
        }
        0x06 => {
            let bytes = data.get(..6).ok_or(())?;
            if is_datetime {
                Ok(DecodedValue::Str(date_i_decode(
                    bytes.try_into().map_err(|_| ())?,
                )))
            } else {
                Ok(DecodedValue::Real(int_decode(bytes)))
            }
        }
        0x07 => Ok(DecodedValue::Real(int_decode(data.get(..8).ok_or(())?))),
        0x09 => Ok(DecodedValue::Real(bcd_decode(data.get(..1).ok_or(())?))),
        0x0A => Ok(DecodedValue::Real(bcd_decode(data.get(..2).ok_or(())?))),
        0x0B => Ok(DecodedValue::Real(bcd_decode(data.get(..3).ok_or(())?))),
        0x0C => Ok(DecodedValue::Real(bcd_decode(data.get(..4).ok_or(())?))),
        0x0D => {
            let lvar = *data.first().ok_or(())?;
            let tail = data.get(1..).ok_or(())?;
            if lvar <= 0xBF {
                Ok(DecodedValue::Str(str_decode(tail)))
            } else {
                Ok(DecodedValue::Str(bin_decode(tail)))
            }
        }
        0x0E => Ok(DecodedValue::Real(bcd_decode(data.get(..6).ok_or(())?))),
        0x0F => Ok(DecodedValue::Str(bin_decode(data))),
        _ => Err(()),
    }
}

/// Little-endian signed integer decode, ported from `mbus_data_int_decode`.
fn int_decode(bytes: &[u8]) -> f64 {
    let neg = bytes.last().is_some_and(|byte| byte & 0x80 != 0);
    let mut value: i64 = 0;
    for &byte in bytes.iter().rev() {
        let byte = if neg { byte ^ 0xFF } else { byte };
        value = value.wrapping_shl(8).wrapping_add(i64::from(byte));
    }
    if neg {
        value = -value - 1;
    }
    value as f64
}

/// BCD decode, ported from `mbus_data_bcd_decode` including its handling of
/// invalid high nibbles and the negative-number F marker.
fn bcd_decode(bytes: &[u8]) -> f64 {
    let mut value: i64 = 0;
    for &byte in bytes.iter().rev() {
        value *= 10;
        if byte >> 4 < 0xA {
            value += i64::from(byte >> 4);
        }
        value = value * 10 + i64::from(byte & 0x0F);
    }
    if bytes.last().is_some_and(|byte| byte >> 4 == 0xF) {
        value = -value;
    }
    value as f64
}

/// Reversed ASCII string decode (`mbus_data_str_decode`).
fn str_decode(bytes: &[u8]) -> String {
    let reversed: Vec<u8> = bytes.iter().rev().copied().collect();
    String::from_utf8_lossy(&reversed).into_owned()
}

/// Space-separated hex dump (`mbus_data_bin_decode`).
///
/// Kept in frame byte order: the reference `.norm.xml` corpus was generated
/// before libmbus started printing binary data LSB-first (commit 9b4b824).
fn bin_decode(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{:02X}", byte))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Type G date (2 bytes), printed like libmbus even when the fields are
/// out of range (e.g. "2000-00-00").
fn date_g_decode(bytes: &[u8; 2]) -> String {
    let day = bytes[0] & 0x1F;
    let month = bytes[1] & 0x0F;
    let year = 2000 + u32::from(((bytes[0] & 0xE0) >> 5) | ((bytes[1] & 0xF0) >> 1));
    format!("{:04}-{:02}-{:02}", year, month, day)
}

/// Type F date and time (4 bytes).
fn date_f_decode(bytes: &[u8; 4]) -> String {
    if bytes[0] & 0x80 != 0 {
        // "time invalid" flag: libmbus leaves the zeroed struct tm in place.
        return "1900-01-00T00:00:00Z".to_string();
    }
    let minute = bytes[0] & 0x3F;
    let hour = bytes[1] & 0x1F;
    let day = bytes[2] & 0x1F;
    let month = bytes[3] & 0x0F;
    // Always century 20xx: the reference corpus predates libmbus's
    // hundred-year detection (commit 7d4e89f).
    let year = 2000 + u32::from(((bytes[2] & 0xE0) >> 5) | ((bytes[3] & 0xF0) >> 1));
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:00Z",
        year, month, day, hour, minute
    )
}

/// Type I date and time (6 bytes).
fn date_i_decode(bytes: &[u8; 6]) -> String {
    if bytes[1] & 0x80 != 0 {
        return "1900-01-00T00:00:00Z".to_string();
    }
    let second = bytes[0] & 0x3F;
    let minute = bytes[1] & 0x3F;
    let hour = bytes[2] & 0x1F;
    let day = bytes[3] & 0x1F;
    let month = bytes[4] & 0x0F;
    let year = 2000 + u32::from(((bytes[3] & 0xE0) >> 5) | ((bytes[4] & 0xF0) >> 1));
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hour, minute, second
    )
}

fn vib_unit_normalize(
    vif: u8,
    vifes: &[u8],
    custom_vif: &Option<String>,
    value: f64,
) -> Option<(String, f64, String)> {
    let (unit, mut normalized, quantity) = if vif == 0xFD {
        let vife = vifes.first()?;
        vif_unit_normalize(u16::from(vife & 0x7F) | 0x100, value)?
    } else if vif == 0xFB {
        let vife = vifes.first()?;
        vif_unit_normalize(u16::from(vife & 0x7F) | 0x200, value)?
    } else if vif == 0x7C || vif == 0xFC {
        (
            "-".to_string(),
            value,
            custom_vif.clone().unwrap_or_default(),
        )
    } else {
        vif_unit_normalize(u16::from(vif & 0x7F), value)?
    };

    // VIFE correction codes (EN 13757 table 8.4.5), applied to the first VIFE.
    if vif & 0x80 != 0 && vif != 0xFD && vif != 0xFB {
        if let Some(&vife) = vifes.first() {
            match vife & 0x7F {
                0x70..=0x77 => {
                    normalized *= 10f64.powi(i32::from(vife & 0x07) - 6);
                }
                0x78..=0x7B => {
                    normalized += 10f64.powi(i32::from(vife & 0x03) - 3);
                }
                0x7D => {
                    normalized *= 1000.0;
                }
                _ => {}
            }
        }
    }

    Some((unit, normalized, quantity))
}

fn vif_unit_normalize(code: u16, value: f64) -> Option<(String, f64, String)> {
    let code = code & 0xF7F;
    VIF_TABLE
        .iter()
        .find(|(vif, _, _, _)| *vif == code)
        .map(|(_, exponent, unit, quantity)| {
            (
                (*unit).to_string(),
                value * exponent,
                (*quantity).to_string(),
            )
        })
}

fn render_fixed(
    id: u32,
    access_number: u8,
    status: u8,
    device_type_and_unit: u16,
    counter1: &user_data::Counter,
    counter2: &user_data::Counter,
) -> String {
    let cnt1_type = (device_type_and_unit >> 8) as u8;
    let cnt2_type = (device_type_and_unit & 0xFF) as u8;

    let mut out = String::new();
    out.push_str(XML_PROCESSING_INSTRUCTION);
    out.push_str("<MBusData>\n\n");
    out.push_str("    <SlaveInformation>\n");
    out.push_str(&format!("        <Id>{}</Id>\n", id));
    out.push_str(&format!(
        "        <Medium>{}</Medium>\n",
        xml_encode(fixed_medium(cnt1_type, cnt2_type))
    ));
    out.push_str(&format!(
        "        <AccessNumber>{}</AccessNumber>\n",
        access_number
    ));
    out.push_str(&format!("        <Status>{:02X}</Status>\n", status));
    out.push_str("    </SlaveInformation>\n\n");

    let function = if status & 0x40 != 0 {
        "Stored value"
    } else {
        "Actual value"
    };
    for (index, (cnt_type, counter)) in [(cnt1_type, counter1), (cnt2_type, counter2)]
        .iter()
        .enumerate()
    {
        out.push_str(&format!("    <DataRecord id=\"{}\">\n", index));
        out.push_str(&format!("        <Function>{}</Function>\n", function));
        out.push_str(&format!(
            "        <Unit>{}</Unit>\n",
            xml_encode(fixed_unit(*cnt_type))
        ));
        // The parser always decodes the counters as BCD; libmbus prints the
        // digits without leading zeros.
        let count: u64 = counter.to_string().parse().unwrap_or_default();
        out.push_str(&format!("        <Value>{}</Value>\n", count));
        out.push_str("    </DataRecord>\n\n");
    }

    out.push_str("</MBusData>\n");
    out
}

fn fixed_medium(cnt1_type: u8, cnt2_type: u8) -> &'static str {
    match ((cnt1_type & 0xC0) >> 6) | ((cnt2_type & 0xC0) >> 4) {
        0x00 => "Other",
        0x01 => "Oil",
        0x02 => "Electricity",
        0x03 => "Gas",
        0x04 => "Heat",
        0x05 => "Steam",
        0x06 => "Hot Water",
        0x07 => "Water",
        0x08 => "H.C.A.",
        0x09 | 0x0F => "Reserved",
        0x0A => "Gas Mode 2",
        0x0B => "Heat Mode 2",
        0x0C => "Hot Water Mode 2",
        0x0D => "Water Mode 2",
        0x0E => "H.C.A. Mode 2",
        _ => "unknown",
    }
}

fn fixed_unit(medium_unit_byte: u8) -> &'static str {
    match medium_unit_byte & 0x3F {
        0x00 => "h,m,s",
        0x01 => "D,M,Y",
        0x02 => "Wh",
        0x03 => "10 Wh",
        0x04 => "100 Wh",
        0x05 => "kWh",
        0x06 => "10 kWh",
        0x07 => "100 kWh",
        0x08 => "MWh",
        0x09 => "10 MWh",
        0x0A => "100 MWh",
        0x0B => "kJ",
        0x0C => "10 kJ",
        0x0E => "100 kJ",
        0x0D => "MJ",
        0x0F => "10 MJ",
        0x10 => "100 MJ",
        0x11 => "GJ",
        0x12 => "10 GJ",
        0x13 => "100 GJ",
        0x14 => "W",
        0x15 => "10 W",
        0x16 => "100 W",
        0x17 => "kW",
        0x18 => "10 kW",
        0x19 => "100 kW",
        0x1A => "MW",
        0x1B => "10 MW",
        0x1C => "100 MW",
        0x1D => "kJ/h",
        0x1E => "10 kJ/h",
        0x1F => "100 kJ/h",
        0x20 => "MJ/h",
        0x21 => "10 MJ/h",
        0x22 => "100 MJ/h",
        0x23 => "GJ/h",
        0x24 => "10 GJ/h",
        0x25 => "100 GJ/h",
        0x26 => "ml",
        0x27 => "10 ml",
        0x28 => "100 ml",
        0x29 => "l",
        0x2A => "10 l",
        0x2B => "100 l",
        0x2C => "m^3",
        0x2D => "10 m^3",
        0x2E => "100 m^3",
        0x2F => "ml/h",
        0x30 => "10 ml/h",
        0x31 => "100 ml/h",
        0x32 => "l/h",
        0x33 => "10 l/h",
        0x34 => "100 l/h",
        0x35 => "m^3/h",
        0x36 => "10 m^3/h",
        0x37 => "100 m^3/h",
        0x38 => "1e-3 °C",
        0x39 => "units for HCA",
        0x3A..=0x3D => "reserved",
        0x3E => "reserved but historic",
        0x3F => "without units",
        _ => "unknown",
    }
}

fn medium_lookup(medium: u8) -> String {
    match medium {
        0x00 => "Other".to_string(),
        0x01 => "Oil".to_string(),
        0x02 => "Electricity".to_string(),
        0x03 => "Gas".to_string(),
        0x04 => "Heat: Outlet".to_string(),
        0x05 => "Steam".to_string(),
        0x06 => "Warm water (30-90°C)".to_string(),
        0x07 => "Water".to_string(),
        0x08 => "Heat Cost Allocator".to_string(),
        0x09 => "Compressed Air".to_string(),
        0x0A => "Cooling load meter: Outlet".to_string(),
        0x0B => "Cooling load meter: Inlet".to_string(),
        0x0C => "Heat: Inlet".to_string(),
        0x0D => "Heat / Cooling load meter".to_string(),
        0x0E => "Bus/System".to_string(),
        0x0F => "Unknown Medium".to_string(),
        0x10 => "Irrigation Water".to_string(),
        0x11 => "Water Logger".to_string(),
        0x12 => "Gas Logger".to_string(),
        0x13 => "Gas Converter".to_string(),
        0x14 => "Calorific value".to_string(),
        0x15 => "Hot water (>90°C)".to_string(),
        0x16 => "Cold water".to_string(),
        0x17 => "Dual water".to_string(),
        0x18 => "Pressure".to_string(),
        0x19 => "A/D Converter".to_string(),
        0x1A => "Smoke Detector".to_string(),
        0x1B => "Ambient Sensor".to_string(),
        0x1C => "Gas Detector".to_string(),
        0x20 => "Breaker: Electricity".to_string(),
        0x21 => "Valve: Gas or Water".to_string(),
        0x25 => "Customer Unit: Display Device".to_string(),
        0x28 => "Waste Water".to_string(),
        0x29 => "Garbage".to_string(),
        0x30 => "Service Unit".to_string(),
        0x36 => "Radio Converter: System".to_string(),
        0x37 => "Radio Converter: Meter".to_string(),
        0x22..=0x24 | 0x26 | 0x27 | 0x2A..=0x2F | 0x31..=0x34 | 0x38..=0x3F => {
            "Reserved".to_string()
        }
        _ => format!("Unknown medium (0x{:02x})", medium),
    }
}

fn product_name(manufacturer: &str, version: u8, medium: u8, id: u32) -> String {
    // Two most significant BCD digits of the ID (libmbus header->id_bcd[3]).
    let id_msb = (((id / 10_000_000) % 10) << 4 | ((id / 1_000_000) % 10)) as u8;
    let name = match manufacturer {
        "ABB" => match version {
            0x02 => "ABB Delta-Meter",
            0x20 => "ABB B21 113-100",
            _ => "",
        },
        "ACW" => match version {
            0x09 => "Itron CF Echo 2",
            0x0A => "Itron CF 51",
            0x0B => "Itron CF 55",
            0x0E => "Itron BM +m",
            0x0F => "Itron CF 800",
            0x14 => "Itron CYBLE M-Bus 1.4",
            _ => "",
        },
        "AMT" => {
            if version >= 0xC0 {
                "Aquametro CALEC ST"
            } else if version >= 0x80 {
                "Aquametro CALEC MB"
            } else if version >= 0x40 {
                "Aquametro SAPHIR"
            } else {
                "Aquametro AMTRON"
            }
        }
        "BEC" => match (medium, version) {
            (0x02, 0x00) => "Berg DCMi",
            (0x02, 0x07) => "Berg BLMi",
            (0x0F, 0x71) => "Berg BMB-10S0",
            _ => "",
        },
        "EFE" => match version {
            0x00 => {
                if medium == 0x06 {
                    "Engelmann WaterStar"
                } else {
                    "Engelmann / Elster SensoStar 2"
                }
            }
            0x01 => "Engelmann SensoStar 2C",
            _ => "",
        },
        "ELS" => match version {
            0x02 => "Elster TMP-A",
            0x0A => "Elster Falcon",
            0x2F => "Elster F96 Plus",
            _ => "",
        },
        "ELV" => match version {
            0x14..=0x1D => "Elvaco CMa10",
            0x32..=0x3B => "Elvaco CMa11",
            _ => "",
        },
        "EMH" => match version {
            0x00 => "EMH DIZ",
            _ => "",
        },
        "EMU" => match (medium, version) {
            (0x02, 0x10) => "EMU Professional 3/75 M-Bus",
            _ => "",
        },
        "GAV" => match (medium, version) {
            (0x02, 0x2D..=0x30) => "Carlo Gavazzi EM24",
            (0x02, 0x39 | 0x3A) => "Carlo Gavazzi EM21",
            (0x02, 0x40) => "Carlo Gavazzi EM33",
            _ => "",
        },
        "GMC" => match version {
            0xE6 => "GMC-I A230 EMMOD 206",
            _ => "",
        },
        "KAM" => match version {
            0x01 => "Kamstrup 382 (6850-005)",
            0x08 => "Kamstrup Multical 601",
            _ => "",
        },
        "SLB" => match version {
            0x02 => "Allmess Megacontrol CF-50",
            0x06 => "CF Compact / Integral MK MaXX",
            _ => "",
        },
        "HYD" => match version {
            0x28 => "ABB F95 Typ US770",
            0x2F => "Hydrometer Sharky 775",
            _ => "",
        },
        "JAN" => match (medium, version) {
            (0x02, 0x09) => "Janitza UMG 96S",
            _ => "",
        },
        "LUG" => match version {
            0x02 => "Landis & Gyr Ultraheat 2WR5",
            0x03 => "Landis & Gyr Ultraheat 2WR6",
            0x04 => "Landis & Gyr Ultraheat UH50",
            0x07 => "Landis & Gyr Ultraheat T230",
            _ => "",
        },
        "LSE" => match version {
            0x99 => "Siemens WFH21",
            _ => "",
        },
        "NZR" => match version {
            0x01 => "NZR DHZ 5/63",
            0x50 => "NZR IC-M2",
            _ => "",
        },
        "RAM" => match version {
            0x03 => "Rossweiner ETK/ETW Modularis",
            _ => "",
        },
        "REL" => match version {
            0x08 => "Relay PadPuls M1",
            0x12 => "Relay PadPuls M4",
            0x20 => "Relay Padin 4",
            0x30 => "Relay AnDi 4",
            0x40 => "Relay PadPuls M2",
            _ => "",
        },
        "RKE" => match version {
            0x69 => "Ista sensonic II mbus",
            _ => "",
        },
        "SBC" => match id_msb {
            0x10 | 0x19 => "Saia-Burgess ALE3",
            0x11 => "Saia-Burgess AWD3",
            _ => "",
        },
        "SEO" | "GTE" => match id_msb {
            0x30 => "Sensoco PT100",
            0x41 => "Sensoco 2-NTC",
            0x45 => "Sensoco Laser Light",
            0x48 => "Sensoco ADIO",
            0x51 | 0x61 => "Sensoco THU",
            0x80 => "Sensoco PulseCounter for E-Meter",
            _ => "",
        },
        "SEN" => match version {
            0x08 | 0x19 => "Sensus PolluCom E",
            0x0B => "Sensus PolluTherm",
            0x0E => "Sensus PolluStat E",
            _ => "",
        },
        "SON" => match version {
            0x0D => "Sontex Supercal 531",
            _ => "",
        },
        "SPX" => match version {
            0x31 | 0x34 => "Sensus PolluTherm",
            _ => "",
        },
        "SVM" => match version {
            0x08 => "Elster F2 / Deltamess F2",
            0x09 => "Elster F4 / Kamstrup SVM F22",
            _ => "",
        },
        "TCH" => match version {
            0x26 => "Techem m-bus S",
            0x40 => "Techem ultra S3",
            _ => "",
        },
        "WZG" => match version {
            0x03 => "Modularis ETW-EAX",
            _ => "",
        },
        "ZRM" => match version {
            0x81 => "Minol Minocal C2",
            0x82 => "Minol Minocal WR3",
            _ => "",
        },
        _ => "",
    };
    name.to_string()
}

/// XML text encoding ported from `mbus_str_xml_encode`: control characters
/// become spaces, `& < > "` are escaped, everything else passes through.
fn xml_encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        if c.is_ascii_control() {
            out.push(' ');
        } else {
            match c {
                '&' => out.push_str("&amp;"),
                '<' => out.push_str("&lt;"),
                '>' => out.push_str("&gt;"),
                '"' => out.push_str("&quot;"),
                _ => out.push(c),
            }
        }
    }
    out
}

/// Normalization table ported from libmbus `vif_table`:
/// (vif code, multiplier, unit, quantity).
#[allow(clippy::unreadable_literal)]
const VIF_TABLE: &[(u16, f64, &str, &str)] = &[
    (0x00, 1.0e-3, "Wh", "Energy"),
    (0x01, 1.0e-2, "Wh", "Energy"),
    (0x02, 1.0e-1, "Wh", "Energy"),
    (0x03, 1.0e0, "Wh", "Energy"),
    (0x04, 1.0e1, "Wh", "Energy"),
    (0x05, 1.0e2, "Wh", "Energy"),
    (0x06, 1.0e3, "Wh", "Energy"),
    (0x07, 1.0e4, "Wh", "Energy"),
    (0x08, 1.0e0, "J", "Energy"),
    (0x09, 1.0e1, "J", "Energy"),
    (0x0A, 1.0e2, "J", "Energy"),
    (0x0B, 1.0e3, "J", "Energy"),
    (0x0C, 1.0e4, "J", "Energy"),
    (0x0D, 1.0e5, "J", "Energy"),
    (0x0E, 1.0e6, "J", "Energy"),
    (0x0F, 1.0e7, "J", "Energy"),
    (0x10, 1.0e-6, "m^3", "Volume"),
    (0x11, 1.0e-5, "m^3", "Volume"),
    (0x12, 1.0e-4, "m^3", "Volume"),
    (0x13, 1.0e-3, "m^3", "Volume"),
    (0x14, 1.0e-2, "m^3", "Volume"),
    (0x15, 1.0e-1, "m^3", "Volume"),
    (0x16, 1.0e0, "m^3", "Volume"),
    (0x17, 1.0e1, "m^3", "Volume"),
    (0x18, 1.0e-3, "kg", "Mass"),
    (0x19, 1.0e-2, "kg", "Mass"),
    (0x1A, 1.0e-1, "kg", "Mass"),
    (0x1B, 1.0e0, "kg", "Mass"),
    (0x1C, 1.0e1, "kg", "Mass"),
    (0x1D, 1.0e2, "kg", "Mass"),
    (0x1E, 1.0e3, "kg", "Mass"),
    (0x1F, 1.0e4, "kg", "Mass"),
    (0x20, 1.0, "s", "On time"),
    (0x21, 60.0, "s", "On time"),
    (0x22, 3600.0, "s", "On time"),
    (0x23, 86400.0, "s", "On time"),
    (0x24, 1.0, "s", "Operating time"),
    (0x25, 60.0, "s", "Operating time"),
    (0x26, 3600.0, "s", "Operating time"),
    (0x27, 86400.0, "s", "Operating time"),
    (0x28, 1.0e-3, "W", "Power"),
    (0x29, 1.0e-2, "W", "Power"),
    (0x2A, 1.0e-1, "W", "Power"),
    (0x2B, 1.0e0, "W", "Power"),
    (0x2C, 1.0e1, "W", "Power"),
    (0x2D, 1.0e2, "W", "Power"),
    (0x2E, 1.0e3, "W", "Power"),
    (0x2F, 1.0e4, "W", "Power"),
    (0x30, 1.0e0, "J/h", "Power"),
    (0x31, 1.0e1, "J/h", "Power"),
    (0x32, 1.0e2, "J/h", "Power"),
    (0x33, 1.0e3, "J/h", "Power"),
    (0x34, 1.0e4, "J/h", "Power"),
    (0x35, 1.0e5, "J/h", "Power"),
    (0x36, 1.0e6, "J/h", "Power"),
    (0x37, 1.0e7, "J/h", "Power"),
    (0x38, 1.0e-6, "m^3/h", "Volume flow"),
    (0x39, 1.0e-5, "m^3/h", "Volume flow"),
    (0x3A, 1.0e-4, "m^3/h", "Volume flow"),
    (0x3B, 1.0e-3, "m^3/h", "Volume flow"),
    (0x3C, 1.0e-2, "m^3/h", "Volume flow"),
    (0x3D, 1.0e-1, "m^3/h", "Volume flow"),
    (0x3E, 1.0e0, "m^3/h", "Volume flow"),
    (0x3F, 1.0e1, "m^3/h", "Volume flow"),
    (0x40, 1.0e-7, "m^3/min", "Volume flow"),
    (0x41, 1.0e-6, "m^3/min", "Volume flow"),
    (0x42, 1.0e-5, "m^3/min", "Volume flow"),
    (0x43, 1.0e-4, "m^3/min", "Volume flow"),
    (0x44, 1.0e-3, "m^3/min", "Volume flow"),
    (0x45, 1.0e-2, "m^3/min", "Volume flow"),
    (0x46, 1.0e-1, "m^3/min", "Volume flow"),
    (0x47, 1.0e0, "m^3/min", "Volume flow"),
    (0x48, 1.0e-9, "m^3/s", "Volume flow"),
    (0x49, 1.0e-8, "m^3/s", "Volume flow"),
    (0x4A, 1.0e-7, "m^3/s", "Volume flow"),
    (0x4B, 1.0e-6, "m^3/s", "Volume flow"),
    (0x4C, 1.0e-5, "m^3/s", "Volume flow"),
    (0x4D, 1.0e-4, "m^3/s", "Volume flow"),
    (0x4E, 1.0e-3, "m^3/s", "Volume flow"),
    (0x4F, 1.0e-2, "m^3/s", "Volume flow"),
    (0x50, 1.0e-3, "kg/h", "Mass flow"),
    (0x51, 1.0e-2, "kg/h", "Mass flow"),
    (0x52, 1.0e-1, "kg/h", "Mass flow"),
    (0x53, 1.0e0, "kg/h", "Mass flow"),
    (0x54, 1.0e1, "kg/h", "Mass flow"),
    (0x55, 1.0e2, "kg/h", "Mass flow"),
    (0x56, 1.0e3, "kg/h", "Mass flow"),
    (0x57, 1.0e4, "kg/h", "Mass flow"),
    (0x58, 1.0e-3, "°C", "Flow temperature"),
    (0x59, 1.0e-2, "°C", "Flow temperature"),
    (0x5A, 1.0e-1, "°C", "Flow temperature"),
    (0x5B, 1.0e0, "°C", "Flow temperature"),
    (0x5C, 1.0e-3, "°C", "Return temperature"),
    (0x5D, 1.0e-2, "°C", "Return temperature"),
    (0x5E, 1.0e-1, "°C", "Return temperature"),
    (0x5F, 1.0e0, "°C", "Return temperature"),
    (0x60, 1.0e-3, "K", "Temperature difference"),
    (0x61, 1.0e-2, "K", "Temperature difference"),
    (0x62, 1.0e-1, "K", "Temperature difference"),
    (0x63, 1.0e0, "K", "Temperature difference"),
    (0x64, 1.0e-3, "°C", "External temperature"),
    (0x65, 1.0e-2, "°C", "External temperature"),
    (0x66, 1.0e-1, "°C", "External temperature"),
    (0x67, 1.0e0, "°C", "External temperature"),
    (0x68, 1.0e-3, "bar", "Pressure"),
    (0x69, 1.0e-2, "bar", "Pressure"),
    (0x6A, 1.0e-1, "bar", "Pressure"),
    (0x6B, 1.0e0, "bar", "Pressure"),
    (0x6C, 1.0e0, "-", "Time point (date)"),
    (0x6D, 1.0e0, "-", "Time point (date & time)"),
    (0x6E, 1.0e0, "Units for H.C.A.", "H.C.A."),
    (0x6F, 0.0, "Reserved", "Reserved"),
    (0x70, 1.0, "s", "Averaging Duration"),
    (0x71, 60.0, "s", "Averaging Duration"),
    (0x72, 3600.0, "s", "Averaging Duration"),
    (0x73, 86400.0, "s", "Averaging Duration"),
    (0x74, 1.0, "s", "Actuality Duration"),
    (0x75, 60.0, "s", "Actuality Duration"),
    (0x76, 3600.0, "s", "Actuality Duration"),
    (0x77, 86400.0, "s", "Actuality Duration"),
    (0x78, 1.0, "", "Fabrication No"),
    (0x79, 1.0, "", "(Enhanced) Identification"),
    (0x7A, 1.0, "", "Bus Address"),
    (0x7E, 1.0, "", "Any VIF"),
    (0x7F, 1.0, "", "Manufacturer specific"),
    (0xFE, 1.0, "", "Any VIF"),
    (0xFF, 1.0, "", "Manufacturer specific"),
    (0x100, 1.0e-3, "Currency units", "Credit"),
    (0x101, 1.0e-2, "Currency units", "Credit"),
    (0x102, 1.0e-1, "Currency units", "Credit"),
    (0x103, 1.0e0, "Currency units", "Credit"),
    (0x104, 1.0e-3, "Currency units", "Debit"),
    (0x105, 1.0e-2, "Currency units", "Debit"),
    (0x106, 1.0e-1, "Currency units", "Debit"),
    (0x107, 1.0e0, "Currency units", "Debit"),
    (0x108, 1.0e0, "", "Access Number (transmission count)"),
    (0x109, 1.0e0, "", "Medium"),
    (0x10A, 1.0e0, "", "Manufacturer"),
    (0x10B, 1.0e0, "", "Parameter set identification"),
    (0x10C, 1.0e0, "", "Model / Version"),
    (0x10D, 1.0e0, "", "Hardware version"),
    (0x10E, 1.0e0, "", "Firmware version"),
    (0x10F, 1.0e0, "", "Software version"),
    (0x110, 1.0e0, "", "Customer location"),
    (0x111, 1.0e0, "", "Customer"),
    (0x112, 1.0e0, "", "Access Code User"),
    (0x113, 1.0e0, "", "Access Code Operator"),
    (0x114, 1.0e0, "", "Access Code System Operator"),
    (0x115, 1.0e0, "", "Access Code Developer"),
    (0x116, 1.0e0, "", "Password"),
    (0x117, 1.0e0, "", "Error flags"),
    (0x118, 1.0e0, "", "Error mask"),
    (0x119, 1.0e0, "Reserved", "Reserved"),
    (0x11A, 1.0e0, "", "Digital Output"),
    (0x11B, 1.0e0, "", "Digital Input"),
    (0x11C, 1.0e0, "Baud", "Baudrate"),
    (0x11D, 1.0e0, "Bittimes", "Response delay time"),
    (0x11E, 1.0e0, "", "Retry"),
    (0x11F, 1.0e0, "Reserved", "Reserved"),
    (0x120, 1.0e0, "", "First storage # for cyclic storage"),
    (0x121, 1.0e0, "", "Last storage # for cyclic storage"),
    (0x122, 1.0e0, "", "Size of storage block"),
    (0x123, 1.0e0, "Reserved", "Reserved"),
    (0x124, 1.0, "s", "Storage interval"),
    (0x125, 60.0, "s", "Storage interval"),
    (0x126, 3600.0, "s", "Storage interval"),
    (0x127, 86400.0, "s", "Storage interval"),
    (0x128, 2629743.83, "s", "Storage interval"),
    (0x129, 31556926.0, "s", "Storage interval"),
    (0x12A, 1.0e0, "Reserved", "Reserved"),
    (0x12B, 1.0e0, "Reserved", "Reserved"),
    (0x12C, 1.0, "s", "Duration since last readout"),
    (0x12D, 60.0, "s", "Duration since last readout"),
    (0x12E, 3600.0, "s", "Duration since last readout"),
    (0x12F, 86400.0, "s", "Duration since last readout"),
    (0x130, 1.0e0, "Reserved", "Reserved"),
    (0x131, 60.0, "s", "Duration of tariff"),
    (0x132, 3600.0, "s", "Duration of tariff"),
    (0x133, 86400.0, "s", "Duration of tariff"),
    (0x134, 1.0, "s", "Period of tariff"),
    (0x135, 60.0, "s", "Period of tariff"),
    (0x136, 3600.0, "s", "Period of tariff"),
    (0x137, 86400.0, "s", "Period of tariff"),
    (0x138, 2629743.83, "s", "Period of tariff"),
    (0x139, 31556926.0, "s", "Period of tariff"),
    (0x13A, 1.0e0, "", "Dimensionless"),
    (0x13B, 1.0e0, "Reserved", "Reserved"),
    (0x13C, 1.0e0, "Reserved", "Reserved"),
    (0x13D, 1.0e0, "Reserved", "Reserved"),
    (0x13E, 1.0e0, "Reserved", "Reserved"),
    (0x13F, 1.0e0, "Reserved", "Reserved"),
    (0x140, 1.0e-9, "V", "Voltage"),
    (0x141, 1.0e-8, "V", "Voltage"),
    (0x142, 1.0e-7, "V", "Voltage"),
    (0x143, 1.0e-6, "V", "Voltage"),
    (0x144, 1.0e-5, "V", "Voltage"),
    (0x145, 1.0e-4, "V", "Voltage"),
    (0x146, 1.0e-3, "V", "Voltage"),
    (0x147, 1.0e-2, "V", "Voltage"),
    (0x148, 1.0e-1, "V", "Voltage"),
    (0x149, 1.0e0, "V", "Voltage"),
    (0x14A, 1.0e1, "V", "Voltage"),
    (0x14B, 1.0e2, "V", "Voltage"),
    (0x14C, 1.0e3, "V", "Voltage"),
    (0x14D, 1.0e4, "V", "Voltage"),
    (0x14E, 1.0e5, "V", "Voltage"),
    (0x14F, 1.0e6, "V", "Voltage"),
    (0x150, 1.0e-12, "A", "Current"),
    (0x151, 1.0e-11, "A", "Current"),
    (0x152, 1.0e-10, "A", "Current"),
    (0x153, 1.0e-9, "A", "Current"),
    (0x154, 1.0e-8, "A", "Current"),
    (0x155, 1.0e-7, "A", "Current"),
    (0x156, 1.0e-6, "A", "Current"),
    (0x157, 1.0e-5, "A", "Current"),
    (0x158, 1.0e-4, "A", "Current"),
    (0x159, 1.0e-3, "A", "Current"),
    (0x15A, 1.0e-2, "A", "Current"),
    (0x15B, 1.0e-1, "A", "Current"),
    (0x15C, 1.0e0, "A", "Current"),
    (0x15D, 1.0e1, "A", "Current"),
    (0x15E, 1.0e2, "A", "Current"),
    (0x15F, 1.0e3, "A", "Current"),
    (0x160, 1.0e0, "", "Reset counter"),
    (0x161, 1.0e0, "", "Cumulation counter"),
    (0x162, 1.0e0, "", "Control signal"),
    (0x163, 1.0e0, "", "Day of week"),
    (0x164, 1.0e0, "", "Week number"),
    (0x165, 1.0e0, "", "Time point of day change"),
    (0x166, 1.0e0, "", "State of parameter activation"),
    (0x167, 1.0e0, "", "Special supplier information"),
    (0x168, 3600.0, "s", "Duration since last cumulation"),
    (0x169, 86400.0, "s", "Duration since last cumulation"),
    (0x16A, 2629743.83, "s", "Duration since last cumulation"),
    (0x16B, 31556926.0, "s", "Duration since last cumulation"),
    (0x16C, 3600.0, "s", "Operating time battery"),
    (0x16D, 86400.0, "s", "Operating time battery"),
    (0x16E, 2629743.83, "s", "Operating time battery"),
    (0x16F, 31556926.0, "s", "Operating time battery"),
    (0x170, 1.0e0, "", "Date and time of battery change"),
    (0x171, 1.0e0, "Reserved", "Reserved"),
    (0x172, 1.0e0, "Reserved", "Reserved"),
    (0x173, 1.0e0, "Reserved", "Reserved"),
    (0x174, 1.0e0, "Reserved", "Reserved"),
    (0x175, 1.0e0, "Reserved", "Reserved"),
    (0x176, 1.0e0, "Reserved", "Reserved"),
    (0x177, 1.0e0, "Reserved", "Reserved"),
    (0x178, 1.0e0, "Reserved", "Reserved"),
    (0x179, 1.0e0, "Reserved", "Reserved"),
    (0x17A, 1.0e0, "Reserved", "Reserved"),
    (0x17B, 1.0e0, "Reserved", "Reserved"),
    (0x17C, 1.0e0, "Reserved", "Reserved"),
    (0x17D, 1.0e0, "Reserved", "Reserved"),
    (0x17E, 1.0e0, "Reserved", "Reserved"),
    (0x17F, 1.0e0, "Reserved", "Reserved"),
    (0x200, 1.0e5, "Wh", "Energy"),
    (0x201, 1.0e6, "Wh", "Energy"),
    (0x202, 1.0e0, "Reserved", "Reserved"),
    (0x203, 1.0e0, "Reserved", "Reserved"),
    (0x204, 1.0e0, "Reserved", "Reserved"),
    (0x205, 1.0e0, "Reserved", "Reserved"),
    (0x206, 1.0e0, "Reserved", "Reserved"),
    (0x207, 1.0e0, "Reserved", "Reserved"),
    (0x208, 1.0e8, "Reserved", "Energy"),
    (0x209, 1.0e9, "Reserved", "Energy"),
    (0x20A, 1.0e0, "Reserved", "Reserved"),
    (0x20B, 1.0e0, "Reserved", "Reserved"),
    (0x20C, 1.0e0, "Reserved", "Reserved"),
    (0x20D, 1.0e0, "Reserved", "Reserved"),
    (0x20E, 1.0e0, "Reserved", "Reserved"),
    (0x20F, 1.0e0, "Reserved", "Reserved"),
    (0x210, 1.0e2, "m^3", "Volume"),
    (0x211, 1.0e3, "m^3", "Volume"),
    (0x212, 1.0e0, "Reserved", "Reserved"),
    (0x213, 1.0e0, "Reserved", "Reserved"),
    (0x214, 1.0e0, "Reserved", "Reserved"),
    (0x215, 1.0e0, "Reserved", "Reserved"),
    (0x216, 1.0e0, "Reserved", "Reserved"),
    (0x217, 1.0e0, "Reserved", "Reserved"),
    (0x218, 1.0e5, "kg", "Mass"),
    (0x219, 1.0e6, "kg", "Mass"),
    (0x21A, 1.0e0, "Reserved", "Reserved"),
    (0x21B, 1.0e0, "Reserved", "Reserved"),
    (0x21C, 1.0e0, "Reserved", "Reserved"),
    (0x21D, 1.0e0, "Reserved", "Reserved"),
    (0x21E, 1.0e0, "Reserved", "Reserved"),
    (0x21F, 1.0e0, "Reserved", "Reserved"),
    (0x220, 1.0e0, "Reserved", "Reserved"),
    (0x221, 1.0e-1, "feet^3", "Volume"),
    (0x222, 1.0e-1, "American gallon", "Volume"),
    (0x223, 1.0e0, "American gallon", "Volume"),
    (0x224, 1.0e-3, "American gallon/min", "Volume flow"),
    (0x225, 1.0e0, "American gallon/min", "Volume flow"),
    (0x226, 1.0e0, "American gallon/h", "Volume flow"),
    (0x227, 1.0e0, "Reserved", "Reserved"),
    (0x228, 1.0e5, "W", "Power"),
    (0x229, 1.0e6, "W", "Power"),
    (0x22A, 1.0e0, "Reserved", "Reserved"),
    (0x22B, 1.0e0, "Reserved", "Reserved"),
    (0x22C, 1.0e0, "Reserved", "Reserved"),
    (0x22D, 1.0e0, "Reserved", "Reserved"),
    (0x22E, 1.0e0, "Reserved", "Reserved"),
    (0x22F, 1.0e0, "Reserved", "Reserved"),
    (0x230, 1.0e8, "J", "Power"),
    (0x231, 1.0e9, "J", "Power"),
    (0x232, 1.0e0, "Reserved", "Reserved"),
    (0x233, 1.0e0, "Reserved", "Reserved"),
    (0x234, 1.0e0, "Reserved", "Reserved"),
    (0x235, 1.0e0, "Reserved", "Reserved"),
    (0x236, 1.0e0, "Reserved", "Reserved"),
    (0x237, 1.0e0, "Reserved", "Reserved"),
    (0x238, 1.0e0, "Reserved", "Reserved"),
    (0x239, 1.0e0, "Reserved", "Reserved"),
    (0x23A, 1.0e0, "Reserved", "Reserved"),
    (0x23B, 1.0e0, "Reserved", "Reserved"),
    (0x23C, 1.0e0, "Reserved", "Reserved"),
    (0x23D, 1.0e0, "Reserved", "Reserved"),
    (0x23E, 1.0e0, "Reserved", "Reserved"),
    (0x23F, 1.0e0, "Reserved", "Reserved"),
    (0x240, 1.0e0, "Reserved", "Reserved"),
    (0x241, 1.0e0, "Reserved", "Reserved"),
    (0x242, 1.0e0, "Reserved", "Reserved"),
    (0x243, 1.0e0, "Reserved", "Reserved"),
    (0x244, 1.0e0, "Reserved", "Reserved"),
    (0x245, 1.0e0, "Reserved", "Reserved"),
    (0x246, 1.0e0, "Reserved", "Reserved"),
    (0x247, 1.0e0, "Reserved", "Reserved"),
    (0x248, 1.0e0, "Reserved", "Reserved"),
    (0x249, 1.0e0, "Reserved", "Reserved"),
    (0x24A, 1.0e0, "Reserved", "Reserved"),
    (0x24B, 1.0e0, "Reserved", "Reserved"),
    (0x24C, 1.0e0, "Reserved", "Reserved"),
    (0x24D, 1.0e0, "Reserved", "Reserved"),
    (0x24E, 1.0e0, "Reserved", "Reserved"),
    (0x24F, 1.0e0, "Reserved", "Reserved"),
    (0x250, 1.0e0, "Reserved", "Reserved"),
    (0x251, 1.0e0, "Reserved", "Reserved"),
    (0x252, 1.0e0, "Reserved", "Reserved"),
    (0x253, 1.0e0, "Reserved", "Reserved"),
    (0x254, 1.0e0, "Reserved", "Reserved"),
    (0x255, 1.0e0, "Reserved", "Reserved"),
    (0x256, 1.0e0, "Reserved", "Reserved"),
    (0x257, 1.0e0, "Reserved", "Reserved"),
    (0x258, 1.0e-3, "°F", "Flow temperature"),
    (0x259, 1.0e-2, "°F", "Flow temperature"),
    (0x25A, 1.0e-1, "°F", "Flow temperature"),
    (0x25B, 1.0e0, "°F", "Flow temperature"),
    (0x25C, 1.0e-3, "°F", "Return temperature"),
    (0x25D, 1.0e-2, "°F", "Return temperature"),
    (0x25E, 1.0e-1, "°F", "Return temperature"),
    (0x25F, 1.0e0, "°F", "Return temperature"),
    (0x260, 1.0e-3, "°F", "Temperature difference"),
    (0x261, 1.0e-2, "°F", "Temperature difference"),
    (0x262, 1.0e-1, "°F", "Temperature difference"),
    (0x263, 1.0e0, "°F", "Temperature difference"),
    (0x264, 1.0e-3, "°F", "External temperature"),
    (0x265, 1.0e-2, "°F", "External temperature"),
    (0x266, 1.0e-1, "°F", "External temperature"),
    (0x267, 1.0e0, "°F", "External temperature"),
    (0x268, 1.0e0, "Reserved", "Reserved"),
    (0x269, 1.0e0, "Reserved", "Reserved"),
    (0x26A, 1.0e0, "Reserved", "Reserved"),
    (0x26B, 1.0e0, "Reserved", "Reserved"),
    (0x26C, 1.0e0, "Reserved", "Reserved"),
    (0x26D, 1.0e0, "Reserved", "Reserved"),
    (0x26E, 1.0e0, "Reserved", "Reserved"),
    (0x26F, 1.0e0, "Reserved", "Reserved"),
    (0x270, 1.0e-3, "°F", "Cold / Warm Temperature Limit"),
    (0x271, 1.0e-2, "°F", "Cold / Warm Temperature Limit"),
    (0x272, 1.0e-1, "°F", "Cold / Warm Temperature Limit"),
    (0x273, 1.0e0, "°F", "Cold / Warm Temperature Limit"),
    (0x274, 1.0e-3, "°C", "Cold / Warm Temperature Limit"),
    (0x275, 1.0e-2, "°C", "Cold / Warm Temperature Limit"),
    (0x276, 1.0e-1, "°C", "Cold / Warm Temperature Limit"),
    (0x277, 1.0e0, "°C", "Cold / Warm Temperature Limit"),
    (0x278, 1.0e-3, "W", "Cumul count max power"),
    (0x279, 1.0e-3, "W", "Cumul count max power"),
    (0x27A, 1.0e-1, "W", "Cumul count max power"),
    (0x27B, 1.0e0, "W", "Cumul count max power"),
    (0x27C, 1.0e1, "W", "Cumul count max power"),
    (0x27D, 1.0e2, "W", "Cumul count max power"),
    (0x27E, 1.0e3, "W", "Cumul count max power"),
    (0x27F, 1.0e4, "W", "Cumul count max power"),
];
