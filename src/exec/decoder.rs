




//작성 필요

use crate::exec::schema::RawPayload;

#[derive(Debug)]
pub enum DecodeError{
    JsonError(serde_json::Error),
    INvalidFormat,
}

impl From<serde_json::Error> for DecodeError{
    fn from(err: serde_json::Error) -> Self{
        DecodeError::JsonError(err)
    }
}

pub fn decode_payload(payload: &[u8]) -> Result<RawPayload, String>{
    if payload.is_empty(){
        return Err("EMPTY_PAYLOAD".to_string());
    }
    serde_json::from_slice::<RawPayload>(payload)
        .map_err(|e| format!("PAYLOAD_DECODE_FAILED: {e}"))
}