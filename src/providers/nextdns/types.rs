use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct NextDNSRecord {
    pub id: String,
    pub domain: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub value: String,
    pub ttl: Option<u32>,
}

#[derive(Serialize)]
pub struct CreateRecordRequest {
    pub domain: String,
    #[serde(rename = "type")]
    pub record_type: String,
    pub value: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ttl: Option<u32>,
}

#[derive(Deserialize, Debug)]
pub struct NextDNSError {
    pub code: String,
    pub message: String,
}
