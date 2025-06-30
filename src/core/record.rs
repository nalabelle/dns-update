#[allow(clippy::upper_case_acronyms)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DNSRecordType {
    A,
    AAAA,
    CNAME,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DNSRecord {
    pub record_type: DNSRecordType,
    pub name: String,
    pub value: String,
    pub ttl: Option<u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    // Simple parser for test purposes
    fn parse_record(line: &str) -> Result<DNSRecord, &'static str> {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() != 2 {
            return Err("Invalid record format");
        }
        let value = parts[0];
        let name = parts[1];
        if value.parse::<std::net::Ipv4Addr>().is_ok() {
            Ok(DNSRecord {
                record_type: DNSRecordType::A,
                name: name.to_string(),
                value: value.to_string(),
                ttl: None,
            })
        } else if value.parse::<std::net::Ipv6Addr>().is_ok() {
            Ok(DNSRecord {
                record_type: DNSRecordType::AAAA,
                name: name.to_string(),
                value: value.to_string(),
                ttl: None,
            })
        } else if value.contains('.') {
            Ok(DNSRecord {
                record_type: DNSRecordType::CNAME,
                name: name.to_string(),
                value: value.to_string(),
                ttl: None,
            })
        } else {
            Err("Unknown record type")
        }
    }

    #[test]
    fn test_parse_a_record() {
        let line = "1.2.3.4 example.com";
        let rec = parse_record(line).unwrap();
        assert_eq!(rec.record_type, DNSRecordType::A);
        assert_eq!(rec.name, "example.com");
        assert_eq!(rec.value, "1.2.3.4");
    }

    #[test]
    fn test_parse_aaaa_record() {
        let line = "2001:db8::1 ipv6.example.com";
        let rec = parse_record(line).unwrap();
        assert_eq!(rec.record_type, DNSRecordType::AAAA);
        assert_eq!(rec.name, "ipv6.example.com");
        assert_eq!(rec.value, "2001:db8::1");
    }

    #[test]
    fn test_parse_cname_record() {
        let line = "target.example.com cname.example.com";
        let rec = parse_record(line).unwrap();
        assert_eq!(rec.record_type, DNSRecordType::CNAME);
        assert_eq!(rec.name, "cname.example.com");
        assert_eq!(rec.value, "target.example.com");
    }

    #[test]
    fn test_parse_invalid_record() {
        let line = "notanip notadomain";
        assert!(parse_record(line).is_err());
        let line2 = "1.2.3.4";
        assert!(parse_record(line2).is_err());
    }

    #[test]
    fn test_process_rewrites_diff() {
        // Simulate diff: old and new sets
        let old = [
            DNSRecord {
                record_type: DNSRecordType::A,
                name: "a.com".into(),
                value: "1.1.1.1".into(),
                ttl: None,
            },
            DNSRecord {
                record_type: DNSRecordType::CNAME,
                name: "b.com".into(),
                value: "c.com".into(),
                ttl: None,
            },
        ];
        let new = [
            DNSRecord {
                record_type: DNSRecordType::A,
                name: "a.com".into(),
                value: "2.2.2.2".into(),
                ttl: None,
            },
            DNSRecord {
                record_type: DNSRecordType::CNAME,
                name: "b.com".into(),
                value: "c.com".into(),
                ttl: None,
            },
            DNSRecord {
                record_type: DNSRecordType::AAAA,
                name: "ipv6.com".into(),
                value: "2001:db8::1".into(),
                ttl: None,
            },
        ];
        let old_set: HashSet<_> = old.iter().collect();
        let new_set: HashSet<_> = new.iter().collect();

        let to_add: Vec<_> = new_set.difference(&old_set).collect();
        let to_remove: Vec<_> = old_set.difference(&new_set).collect();

        assert_eq!(to_add.len(), 2); // new A for a.com and new AAAA
        assert_eq!(to_remove.len(), 1); // old A for a.com
    }
}
