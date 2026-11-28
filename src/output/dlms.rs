use super::{hex_string, ProtocolData};
use m_bus_application_layer::dlms::{Apdu, DlmsPayload, GetDataResult, GetRequest, GetResponse};
use serde_json::{json, Value};

pub(super) fn decode(
    bytes: &[u8],
    padded: bool,
    parsed: &mut ProtocolData,
) -> Option<Result<Value, &'static str>> {
    let service = DlmsPayload::decode(bytes, padded);
    parsed.evidence.dlms_service = Some(service.into());
    if service.valid == Some(false) {
        return Some(Err("invalid DLMS/COSEM service APDU"));
    }
    let tag = service.tag?;
    let apdu = service.apdu?;
    let mut output = serde_json::Map::new();
    output.insert("service".into(), format!("{tag:?}").into());
    output.insert("service_tag".into(), tag.as_u8().into());
    output.insert("sap_hex".into(), hex_string(bytes.get(..2)?).into());
    output.insert("protected".into(), service.protected.into());
    if let Some(invoke) = service.invoke {
        output.insert("invoke_id".into(), (invoke & 0x0f).into());
        output.insert("invoke_id_and_priority".into(), invoke.into());
    }
    match apdu {
        Apdu::GetRequest(GetRequest::Normal { descriptor, .. }) => {
            output.insert("attribute".into(), json!({"class_id":descriptor.class_id,
                "object_name_hex":hex_string(descriptor.instance_id.as_bytes()), "attribute_id":descriptor.attribute_id}));
        }
        Apdu::GetResponse(GetResponse::Normal { result, .. }) => match result {
            GetDataResult::Data(value) => {
                output.insert("data_type".into(), value.tag().as_u8().into());
                if let Some(value) = value
                    .as_i64()
                    .map(Value::from)
                    .or_else(|| value.as_u64().map(Value::from))
                    .or_else(|| value.as_bool().map(Value::from))
                    .or_else(|| value.as_str().map(Value::from))
                {
                    output.insert("value".into(), value);
                }
            }
            GetDataResult::Error(error) => {
                output.insert("access_error".into(), error.as_u8().into());
            }
        },
        _ => {}
    }
    Some(Ok(Value::Object(output)))
}
