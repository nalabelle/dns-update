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

use crate::core::record::{DNSRecord, DNSRecordType};

pub fn to_dns_record(nr: &NextDNSRecord) -> DNSRecord {
    DNSRecord {
        record_type: match nr.record_type.as_str() {
            "A" => DNSRecordType::A,
            "AAAA" => DNSRecordType::AAAA,
            "CNAME" => DNSRecordType::CNAME,
            _ => DNSRecordType::A, // fallback, should handle error
        },
        name: nr.domain.clone(),
        value: nr.value.clone(),
        ttl: nr.ttl,
    }
}

pub fn to_nextdns_record(rec: &DNSRecord) -> CreateRecordRequest {
    CreateRecordRequest {
        domain: rec.name.clone(),
        record_type: match rec.record_type {
            DNSRecordType::A => "A".to_string(),
            DNSRecordType::AAAA => "AAAA".to_string(),
            DNSRecordType::CNAME => "CNAME".to_string(),
        },
        value: rec.value.clone(),
        ttl: rec.ttl,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::record::DNSRecordType;

    // --- Record Conversion Tests ---
    #[test]
    fn test_to_dns_record_and_back() {
        let nextdns = NextDNSRecord {
            id: "abc".to_string(),
            domain: "example.com".to_string(),
            record_type: "A".to_string(),
            value: "1.2.3.4".to_string(),
            ttl: Some(60),
        };
        let dns = to_dns_record(&nextdns);
        assert_eq!(dns.record_type, DNSRecordType::A);
        assert_eq!(dns.name, "example.com");
        assert_eq!(dns.value, "1.2.3.4");
        assert_eq!(dns.ttl, Some(60));

        let req = to_nextdns_record(&dns);
        assert_eq!(req.domain, "example.com");
        assert_eq!(req.record_type, "A");
        assert_eq!(req.value, "1.2.3.4");
        assert_eq!(req.ttl, Some(60));
    }

    #[test]
    fn test_to_dns_record_invalid_type() {
        let nextdns = NextDNSRecord {
            id: "abc".to_string(),
            domain: "example.com".to_string(),
            record_type: "TXT".to_string(),
            value: "foo".to_string(),
            ttl: None,
        };
        let dns = to_dns_record(&nextdns);
        // Fallback is A
        assert_eq!(dns.record_type, DNSRecordType::A);
    }
}
