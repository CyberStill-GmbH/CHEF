use std::net::IpAddr;

use chrono::{DateTime, SecondsFormat, Utc};
use roxmltree::{Document, Node, ParsingOptions};

use crate::domain::{ChefError, ChefResult, ErrorCode, ServiceInput};

pub(crate) fn parse(bytes: &[u8], max_observations: usize) -> ChefResult<Vec<ServiceInput>> {
    let xml = std::str::from_utf8(bytes)
        .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Input must be valid UTF-8"))?;

    // Permit only Nmap's empty, harmless declaration. Reject entity syntax before parsing.
    let xml = xml.replace("<!DOCTYPE nmaprun>", "");
    let upper = xml.to_ascii_uppercase();
    if upper.contains("<!DOCTYPE") || upper.contains("<!ENTITY") {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "External DTDs and entity declarations are disabled",
        ));
    }
    check_xml_depth(&xml)?;

    let document = Document::parse_with_options(
        &xml,
        ParsingOptions {
            allow_dtd: false,
            nodes_limit: max_observations
                .saturating_mul(12)
                .saturating_add(256)
                .min(u32::MAX as usize) as u32,
        },
    )
    .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Malformed or unsupported XML"))?;
    let root = document.root_element();
    if root.tag_name().name() != "nmaprun" || root.attribute("scanner") != Some("nmap") {
        return Err(ChefError::new(ErrorCode::InvalidInput, "Expected Nmap XML"));
    }

    let start = root
        .attribute("start")
        .filter(|value| {
            !value.is_empty() && value.len() <= 11 && value.bytes().all(|b| b.is_ascii_digit())
        })
        .and_then(|value| value.parse::<i64>().ok())
        .ok_or_else(|| {
            ChefError::new(ErrorCode::InvalidInput, "Nmap start must be Unix seconds")
        })?;
    let observed_at = DateTime::<Utc>::from_timestamp(start, 0)
        .ok_or_else(|| ChefError::new(ErrorCode::InvalidInput, "Invalid Nmap timestamp"))?
        .to_rfc3339_opts(SecondsFormat::Millis, true);

    let hosts = root
        .children()
        .filter(|node| node.is_element() && node.tag_name().name() == "host")
        .collect::<Vec<_>>();
    if hosts.len() > max_observations {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Too many host records",
        ));
    }

    let mut output = Vec::new();
    let mut record_count = 0usize;
    for (host_index, host) in hosts.iter().enumerate() {
        let status = child(*host, "status");
        if status.and_then(|node| node.attribute("state")) != Some("up") {
            continue;
        }

        let addresses = children(*host, "address")
            .filter(|node| matches!(node.attribute("addrtype"), Some("ipv4" | "ipv6")))
            .collect::<Vec<_>>();
        if addresses.len() != 1 {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Each imported host must have exactly one IP address",
            ));
        }
        let raw_address = addresses[0]
            .attribute("addr")
            .ok_or_else(|| ChefError::new(ErrorCode::InvalidInput, "Invalid host address"))?;
        let address = raw_address
            .parse::<IpAddr>()
            .map_err(|_| {
                ChefError::new(
                    ErrorCode::InvalidInput,
                    "Expected a literal IP address without zone ID",
                )
            })?
            .to_string();

        let ports = child(*host, "ports")
            .map(|ports| children(ports, "port").collect::<Vec<_>>())
            .unwrap_or_default();
        for (port_index, port) in ports.iter().enumerate() {
            record_count += 1;
            if record_count > max_observations {
                return Err(ChefError::new(
                    ErrorCode::LimitExceeded,
                    "Too many port records",
                ));
            }
            let raw_number = port.attribute("portid").unwrap_or_default();
            if raw_number.is_empty()
                || raw_number.len() > 5
                || !raw_number.bytes().all(|b| b.is_ascii_digit())
            {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Invalid port number",
                ));
            }
            let number = raw_number
                .parse::<u16>()
                .ok()
                .filter(|number| *number > 0)
                .ok_or_else(|| {
                    ChefError::new(ErrorCode::InvalidInput, "Port must be between 1 and 65535")
                })?;
            let protocol = port.attribute("protocol").unwrap_or_default();
            if protocol != "tcp" && protocol != "udp" {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Only TCP and UDP are supported",
                ));
            }
            if child(*port, "state").and_then(|node| node.attribute("state")) != Some("open") {
                continue;
            }
            output.push(ServiceInput {
                address: address.clone(),
                protocol: protocol.to_owned(),
                port: number,
                observed_at: observed_at.clone(),
                locator: format!("/nmaprun/host[{host_index}]/ports/port[{port_index}]"),
            });
        }
    }
    Ok(output)
}

fn check_xml_depth(xml: &str) -> ChefResult<()> {
    let bytes = xml.as_bytes();
    let mut index = 0usize;
    let mut depth = 0usize;
    while index < bytes.len() {
        if bytes[index] != b'<' {
            index += 1;
            continue;
        }
        if bytes[index..].starts_with(b"<!--") {
            index = skip_through(bytes, index + 4, b"-->")?;
            continue;
        }
        if bytes[index..].starts_with(b"<![CDATA[") {
            index = skip_through(bytes, index + 9, b"]]>")?;
            continue;
        }
        if bytes[index..].starts_with(b"<?") {
            index = skip_through(bytes, index + 2, b"?>")?;
            continue;
        }
        let end = find_tag_end(bytes, index + 1)?;
        let body = &bytes[index + 1..end];
        if body.first() == Some(&b'/') {
            depth = depth.saturating_sub(1);
        } else if body.first() == Some(&b'!') {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Unsupported XML declaration",
            ));
        } else if body.iter().rev().find(|byte| !byte.is_ascii_whitespace()) != Some(&b'/') {
            depth += 1;
            if depth > 32 {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "XML exceeds supported nesting depth",
                ));
            }
        }
        index = end + 1;
    }
    Ok(())
}

fn skip_through(bytes: &[u8], mut from: usize, terminator: &[u8]) -> ChefResult<usize> {
    while from + terminator.len() <= bytes.len() {
        if bytes[from..].starts_with(terminator) {
            return Ok(from + terminator.len());
        }
        from += 1;
    }
    Err(ChefError::new(ErrorCode::InvalidInput, "Malformed XML"))
}

fn find_tag_end(bytes: &[u8], mut from: usize) -> ChefResult<usize> {
    let mut quote = None;
    while from < bytes.len() {
        let byte = bytes[from];
        match (quote, byte) {
            (Some(active), current) if active == current => quote = None,
            (None, b'\'' | b'"') => quote = Some(byte),
            (None, b'>') => return Ok(from),
            _ => {}
        }
        from += 1;
    }
    Err(ChefError::new(ErrorCode::InvalidInput, "Malformed XML"))
}

fn child<'a, 'input>(node: Node<'a, 'input>, name: &str) -> Option<Node<'a, 'input>> {
    node.children()
        .find(|child| child.is_element() && child.tag_name().name() == name)
}

fn children<'a, 'input>(
    node: Node<'a, 'input>,
    name: &'a str,
) -> impl Iterator<Item = Node<'a, 'input>> + 'a {
    node.children()
        .filter(move |child| child.is_element() && child.tag_name().name() == name)
}
