//! Extrae identidad de servidor anunciada en un desafío NTLM archivado.
//!
//! El token es evidencia no confiable. El parser no inicia autenticación,
//! valida identidad ni produce hallazgos de configuración o vulnerabilidad.

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_HEADER_CHARS: usize = 16 * 1024;
const MAX_DECODED_BYTES: usize = 12 * 1024;
const MAX_AV_PAIRS: usize = 64;
const MAX_AV_VALUE_BYTES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedNtlmIdentityHints {
    pub netbios_computer_name: Option<String>,
    pub netbios_domain_name: Option<String>,
    pub dns_computer_name: Option<String>,
    pub dns_domain_name: Option<String>,
    pub dns_tree_name: Option<String>,
}

/// Decodifica un desafío NTLM Type 2 desde un encabezado `WWW-Authenticate`.
///
/// Los nombres provienen del servidor archivado y son pistas declarativas, no
/// una comprobación de identidad del host que respondió.
pub fn parse_archived_ntlm_challenge(
    header_value: &str,
) -> ChefResult<Option<ArchivedNtlmIdentityHints>> {
    if header_value.len() > MAX_HEADER_CHARS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El encabezado NTLM archivado supera el límite de bytes",
        ));
    }
    let mut parts = header_value.split_whitespace();
    let first = parts.next().unwrap_or_default();
    let token = if first.eq_ignore_ascii_case("ntlm") || first.eq_ignore_ascii_case("negotiate") {
        parts.next().unwrap_or_default()
    } else {
        first
    };
    if token.is_empty() || parts.next().is_some() {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El encabezado NTLM debe contener un único token",
        ));
    }
    let message = decode_base64(token)?;
    if message.len() < 48 || message.get(..8) != Some(b"NTLMSSP\0") || read_u32(&message, 8)? != 2 {
        return Ok(None);
    }

    let info_length = usize::from(read_u16(&message, 40)?);
    let info_offset = usize::try_from(read_u32(&message, 44)?).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El offset NTLM no es representable",
        )
    })?;
    if info_length == 0 {
        return Ok(None);
    }
    if info_length > MAX_DECODED_BYTES || info_offset < 48 {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El bloque de atributos NTLM supera los límites",
        ));
    }
    let info_end = info_offset.checked_add(info_length).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El bloque de atributos NTLM desborda el mensaje",
    ))?;
    let info = message.get(info_offset..info_end).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El bloque de atributos NTLM está truncado",
    ))?;

    let mut hints = ArchivedNtlmIdentityHints {
        netbios_computer_name: None,
        netbios_domain_name: None,
        dns_computer_name: None,
        dns_domain_name: None,
        dns_tree_name: None,
    };
    let mut cursor = 0;
    let mut pairs_seen = 0;
    let mut terminated = false;
    while cursor < info.len() {
        pairs_seen += 1;
        if pairs_seen > MAX_AV_PAIRS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El desafío NTLM supera el máximo de atributos",
            ));
        }
        let id = read_u16(info, cursor)?;
        let length = usize::from(read_u16(info, cursor + 2)?);
        cursor += 4;
        if id == 0 {
            if length != 0 {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "El terminador de atributos NTLM tiene una longitud inválida",
                ));
            }
            terminated = true;
            break;
        }
        if length > MAX_AV_VALUE_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Un atributo NTLM supera el límite de bytes",
            ));
        }
        let end = cursor.checked_add(length).ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Un atributo NTLM desborda el mensaje",
        ))?;
        let value = info.get(cursor..end).ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "Un atributo NTLM está truncado",
        ))?;
        if let Some(text) = decode_utf16_attribute(value)? {
            match id {
                1 => hints.netbios_computer_name.get_or_insert(text),
                2 => hints.netbios_domain_name.get_or_insert(text),
                3 => hints
                    .dns_computer_name
                    .get_or_insert_with(|| canonical_dns_name(&text).unwrap_or(text)),
                4 => hints
                    .dns_domain_name
                    .get_or_insert_with(|| canonical_dns_name(&text).unwrap_or(text)),
                5 => hints
                    .dns_tree_name
                    .get_or_insert_with(|| canonical_dns_name(&text).unwrap_or(text)),
                _ => continue,
            };
        }
        cursor = end;
    }
    if !terminated {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El bloque de atributos NTLM no tiene terminador",
        ));
    }
    let has_names = hints.netbios_computer_name.is_some()
        || hints.netbios_domain_name.is_some()
        || hints.dns_computer_name.is_some()
        || hints.dns_domain_name.is_some()
        || hints.dns_tree_name.is_some();
    Ok(has_names.then_some(hints))
}

fn decode_base64(token: &str) -> ChefResult<Vec<u8>> {
    if token.len() > MAX_HEADER_CHARS || token.len() % 4 != 0 {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El token NTLM no es base64 válido",
        ));
    }
    let mut output = Vec::with_capacity(token.len() / 4 * 3);
    let chunks = token.as_bytes().chunks_exact(4);
    let chunk_count = chunks.len();
    for (index, chunk) in chunks.enumerate() {
        let final_chunk = index + 1 == chunk_count;
        let a = base64_value(chunk[0]).ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "El token NTLM contiene un carácter base64 inválido",
        ))?;
        let b = base64_value(chunk[1]).ok_or(ChefError::new(
            ErrorCode::InvalidInput,
            "El token NTLM contiene un carácter base64 inválido",
        ))?;
        let c = if chunk[2] == b'=' {
            if !final_chunk || chunk[3] != b'=' {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "El relleno base64 del token NTLM es inválido",
                ));
            }
            None
        } else {
            Some(base64_value(chunk[2]).ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "El token NTLM contiene un carácter base64 inválido",
            ))?)
        };
        let d = if chunk[3] == b'=' {
            if !final_chunk {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "El relleno base64 del token NTLM es inválido",
                ));
            }
            None
        } else {
            Some(base64_value(chunk[3]).ok_or(ChefError::new(
                ErrorCode::InvalidInput,
                "El token NTLM contiene un carácter base64 inválido",
            ))?)
        };
        output.push((a << 2) | (b >> 4));
        if let Some(c) = c {
            output.push((b << 4) | (c >> 2));
            if let Some(d) = d {
                output.push((c << 6) | d);
            }
        }
        if output.len() > MAX_DECODED_BYTES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El mensaje NTLM decodificado supera el límite",
            ));
        }
    }
    Ok(output)
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn read_u16(bytes: &[u8], offset: usize) -> ChefResult<u16> {
    let data = bytes.get(offset..offset + 2).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El mensaje NTLM está truncado",
    ))?;
    Ok(u16::from_le_bytes([data[0], data[1]]))
}

fn read_u32(bytes: &[u8], offset: usize) -> ChefResult<u32> {
    let data = bytes.get(offset..offset + 4).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El mensaje NTLM está truncado",
    ))?;
    Ok(u32::from_le_bytes([data[0], data[1], data[2], data[3]]))
}

fn decode_utf16_attribute(bytes: &[u8]) -> ChefResult<Option<String>> {
    if bytes.is_empty() || bytes.len() % 2 != 0 {
        return Ok(None);
    }
    let units = bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect::<Vec<_>>();
    let value = String::from_utf16(&units).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "Un nombre NTLM no es UTF-16 válido",
        )
    })?;
    let value = value.trim_matches('\0').trim();
    if value.is_empty() || value.chars().any(char::is_control) {
        return Ok(None);
    }
    Ok(Some(value.to_owned()))
}

#[cfg(test)]
mod tests {
    use super::parse_archived_ntlm_challenge;
    use crate::ErrorCode;

    fn append_av_pair(message: &mut Vec<u8>, id: u16, value: &str) {
        let encoded = value
            .encode_utf16()
            .flat_map(u16::to_le_bytes)
            .collect::<Vec<_>>();
        message.extend_from_slice(&id.to_le_bytes());
        message.extend_from_slice(&(encoded.len() as u16).to_le_bytes());
        message.extend_from_slice(&encoded);
    }

    fn encode_base64(bytes: &[u8]) -> String {
        const ALPHABET: &[u8; 64] =
            b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut output = String::new();
        for chunk in bytes.chunks(3) {
            let first = chunk[0];
            let second = *chunk.get(1).unwrap_or(&0);
            let third = *chunk.get(2).unwrap_or(&0);
            output.push(ALPHABET[(first >> 2) as usize] as char);
            output.push(ALPHABET[(((first & 0x03) << 4) | (second >> 4)) as usize] as char);
            if chunk.len() > 1 {
                output.push(ALPHABET[(((second & 0x0f) << 2) | (third >> 6)) as usize] as char);
            } else {
                output.push('=');
            }
            if chunk.len() > 2 {
                output.push(ALPHABET[(third & 0x3f) as usize] as char);
            } else {
                output.push('=');
            }
        }
        output
    }

    fn challenge() -> String {
        let mut message = vec![0; 48];
        message[..8].copy_from_slice(b"NTLMSSP\0");
        message[8..12].copy_from_slice(&2_u32.to_le_bytes());
        let info_offset = message.len() as u32;
        append_av_pair(&mut message, 3, "WEB01.example.com");
        append_av_pair(&mut message, 4, "example.com");
        message.extend_from_slice(&0_u16.to_le_bytes());
        message.extend_from_slice(&0_u16.to_le_bytes());
        let info_length = (message.len() as u32 - info_offset) as u16;
        message[40..42].copy_from_slice(&info_length.to_le_bytes());
        message[42..44].copy_from_slice(&info_length.to_le_bytes());
        message[44..48].copy_from_slice(&info_offset.to_le_bytes());
        encode_base64(&message)
    }

    #[test]
    fn extracts_server_names_from_archived_type_two_challenge() {
        let token = challenge();
        let result = parse_archived_ntlm_challenge(&format!("NTLM {token}"))
            .unwrap()
            .unwrap();
        assert_eq!(
            result.dns_computer_name.as_deref(),
            Some("web01.example.com")
        );
        assert_eq!(result.dns_domain_name.as_deref(), Some("example.com"));
    }

    #[test]
    fn ignores_other_message_types_and_rejects_malformed_tokens() {
        assert!(
            parse_archived_ntlm_challenge("Negotiate YWJjZA==")
                .unwrap()
                .is_none()
        );
        assert_eq!(
            parse_archived_ntlm_challenge("NTLM !!!").unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_header_and_attribute_limits() {
        let oversized = "A".repeat(16 * 1024 + 1);
        assert_eq!(
            parse_archived_ntlm_challenge(&oversized).unwrap_err().code,
            ErrorCode::LimitExceeded
        );
        let mut token = vec![0; 48];
        token[..8].copy_from_slice(b"NTLMSSP\0");
        token[8..12].copy_from_slice(&2_u32.to_le_bytes());
        token[40..42].copy_from_slice(&1_u16.to_le_bytes());
        token[44..48].copy_from_slice(&48_u32.to_le_bytes());
        token.push(b'x');
        assert_eq!(
            parse_archived_ntlm_challenge(&encode_base64(&token))
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
