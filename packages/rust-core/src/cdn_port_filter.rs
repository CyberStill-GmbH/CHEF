//! Evalúa cómo habría actuado la regla de portfilter sobre tags CDN/WAF.
//!
//! El resultado es explicativo: CHEF conserva siempre la evidencia del puerto.
//! No prueba que un host use un CDN, no altera scope ni autoriza tráfico.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode, port_spec::parse_port_string};

const MAX_TAG_SPEC_BYTES: usize = 4 * 1024;
const MAX_CDN_TAGS: usize = 64;
const MAX_TAG_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CdnPortFilterPolicy {
    cdn_tags: BTreeSet<String>,
    allowed_ports: BTreeSet<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CdnPortFilterReason {
    PortIsAllowed,
    NoConfiguredTagMatches,
    MatchingTagAndPortNotAllowed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CdnPortFilterAssessment {
    pub source_filter_would_apply: bool,
    pub reason: CdnPortFilterReason,
    pub evidence_preserved: bool,
}

/// Construye la regla offline de `portfilter` con listas acotadas.
pub fn parse_cdn_port_filter_policy(
    cdn_tag_spec: &str,
    allowed_port_spec: &str,
) -> ChefResult<CdnPortFilterPolicy> {
    if cdn_tag_spec.len() > MAX_TAG_SPEC_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La lista de tags CDN supera el límite CHEF",
        ));
    }
    let mut cdn_tags = BTreeSet::new();
    for tag in cdn_tag_spec
        .split(',')
        .map(str::trim)
        .filter(|tag| !tag.is_empty())
    {
        if tag.len() > MAX_TAG_BYTES || tag.chars().any(char::is_control) {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "Un tag CDN supera el límite CHEF",
            ));
        }
        cdn_tags.insert(tag.to_owned());
        if cdn_tags.len() > MAX_CDN_TAGS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La lista supera el máximo de tags CDN CHEF",
            ));
        }
    }

    if allowed_port_spec.len() > MAX_TAG_SPEC_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La lista de puertos CDN supera el límite CHEF",
        ));
    }
    let allowed_ports = if allowed_port_spec.trim().is_empty() {
        BTreeSet::new()
    } else {
        let normalized = allowed_port_spec
            .split(',')
            .map(str::trim)
            .collect::<Vec<_>>()
            .join(",");
        parse_port_string(&normalized)?.into_iter().collect()
    };
    Ok(CdnPortFilterPolicy {
        cdn_tags,
        allowed_ports,
    })
}

impl CdnPortFilterPolicy {
    /// Explica la regla fuente sin eliminar ni mutar la observación CHEF.
    pub fn assess(&self, port: u16, event_tags: &[String]) -> CdnPortFilterAssessment {
        if self.allowed_ports.contains(&port) {
            return CdnPortFilterAssessment {
                source_filter_would_apply: false,
                reason: CdnPortFilterReason::PortIsAllowed,
                evidence_preserved: true,
            };
        }
        if !event_tags.iter().any(|tag| self.cdn_tags.contains(tag)) {
            return CdnPortFilterAssessment {
                source_filter_would_apply: false,
                reason: CdnPortFilterReason::NoConfiguredTagMatches,
                evidence_preserved: true,
            };
        }
        CdnPortFilterAssessment {
            source_filter_would_apply: true,
            reason: CdnPortFilterReason::MatchingTagAndPortNotAllowed,
            evidence_preserved: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CdnPortFilterReason, parse_cdn_port_filter_policy};
    use crate::ErrorCode;

    #[test]
    fn assesses_cdn_rule_without_discarding_port_evidence() {
        let policy = parse_cdn_port_filter_policy("cdn,waf", "80,443").unwrap();
        let observed_port = 8080;
        let observed_tags = vec!["cdn".to_owned()];
        assert_eq!(
            policy.assess(observed_port, &observed_tags),
            super::CdnPortFilterAssessment {
                source_filter_would_apply: true,
                reason: CdnPortFilterReason::MatchingTagAndPortNotAllowed,
                evidence_preserved: true,
            }
        );
        assert_eq!(observed_port, 8080);
        assert_eq!(observed_tags, ["cdn"]);
        assert_eq!(
            policy.assess(443, &["cdn".to_owned()]).reason,
            CdnPortFilterReason::PortIsAllowed
        );
        assert_eq!(
            policy.assess(8080, &["origin".to_owned()]).reason,
            CdnPortFilterReason::NoConfiguredTagMatches
        );
    }

    #[test]
    fn parses_ranges_and_treats_empty_allowlist_as_no_allowed_ports() {
        let policy = parse_cdn_port_filter_policy("cdn", "80,443-444").unwrap();
        assert!(
            !policy
                .assess(444, &["cdn".to_owned()])
                .source_filter_would_apply
        );
        let empty = parse_cdn_port_filter_policy("cdn", " ").unwrap();
        assert!(
            empty
                .assess(443, &["cdn".to_owned()])
                .source_filter_would_apply
        );
        assert!(
            !empty
                .assess(443, &["other".to_owned()])
                .source_filter_would_apply
        );
    }

    #[test]
    fn rejects_malformed_port_rules_and_excessive_tags() {
        assert_eq!(
            parse_cdn_port_filter_policy("cdn", "80,abc")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        let too_many_tags = (0..65)
            .map(|index| format!("tag{index}"))
            .collect::<Vec<_>>()
            .join(",");
        assert_eq!(
            parse_cdn_port_filter_policy(&too_many_tags, "80")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
