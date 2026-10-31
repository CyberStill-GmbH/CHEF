//! Resume dominios relacionados a partir de un grafo archivado de eventos.
//!
//! Este agregado es contexto histórico para revisar relaciones externas. No
//! atribuye propiedad, no incorpora dominios al scope y no autoriza escaneos.

use std::collections::{BTreeMap, BTreeSet};

use crate::{BbotEvent, ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_EVENTS: usize = 10_000;
const MAX_SCOPE_ROOTS: usize = 32;
const MAX_AFFILIATES: usize = 512;
const SCORE_SCALE: u32 = 1000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffiliateDomainHint {
    pub domain: String,
    /// Puntuación de contexto en milésimas, sin aritmética de coma flotante.
    pub score_milli: u32,
    pub event_count: u32,
}

/// Resume referencias a dominios registrables fuera de las raíces declaradas.
///
/// Adapta la agregación del informe de afiliados de la fuente. Un evento más
/// cercano aporta `1 / scope_distance`; la etiqueta `affiliate` añade una
/// unidad. El valor solo ordena revisión local y no representa confianza.
pub fn summarize_affiliate_domains(
    events: &[BbotEvent],
    scope_roots: &[String],
) -> ChefResult<Vec<AffiliateDomainHint>> {
    if events.len() > MAX_EVENTS || scope_roots.is_empty() || scope_roots.len() > MAX_SCOPE_ROOTS {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Eventos o raíces de scope fuera de los límites del resumen",
        ));
    }
    let mut canonical_roots = BTreeSet::new();
    for root in scope_roots {
        let Some(root) = canonical_dns_name(root) else {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Una raíz de scope no es un nombre DNS válido",
            ));
        };
        canonical_roots.insert(root);
    }

    let mut aggregate: BTreeMap<String, (u32, u32)> = BTreeMap::new();
    for event in events {
        if event.scope_distance == 0 {
            continue;
        }
        let Some(host) = event.host.as_deref().and_then(canonical_dns_name) else {
            continue;
        };
        let Some(domain) = registrable_icann_domain(&host) else {
            continue;
        };
        if is_in_scope(&domain, &canonical_roots) {
            continue;
        }
        let Some(distance_score) = SCORE_SCALE.checked_div(u32::from(event.scope_distance)) else {
            continue;
        };
        let affiliate_bonus = if event.tags.iter().any(|tag| tag == "affiliate") {
            SCORE_SCALE
        } else {
            0
        };
        let (score, count) = aggregate.entry(domain).or_default();
        *score = score.saturating_add(distance_score.saturating_add(affiliate_bonus));
        *count = count.saturating_add(1);
        if aggregate.len() > MAX_AFFILIATES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El grafo supera el máximo de dominios relacionados",
            ));
        }
    }

    let mut hints = aggregate
        .into_iter()
        .map(|(domain, (score_milli, event_count))| AffiliateDomainHint {
            domain,
            score_milli,
            event_count,
        })
        .collect::<Vec<_>>();
    hints.sort_by(|left, right| {
        right
            .score_milli
            .cmp(&left.score_milli)
            .then_with(|| left.domain.cmp(&right.domain))
    });
    Ok(hints)
}

fn registrable_icann_domain(host: &str) -> Option<String> {
    let registrable = psl::domain(host.as_bytes())?;
    (registrable.suffix().typ() == Some(psl::Type::Icann))
        .then(|| String::from_utf8_lossy(registrable.as_bytes()).into_owned())
}

fn is_in_scope(domain: &str, roots: &BTreeSet<String>) -> bool {
    roots
        .iter()
        .any(|root| domain == root || domain.ends_with(&format!(".{root}")))
}

#[cfg(test)]
mod tests {
    use super::summarize_affiliate_domains;
    use crate::BbotEvent;
    use serde_json::json;

    fn event(host: &str, scope_distance: u16, tags: &[&str]) -> BbotEvent {
        BbotEvent {
            id: format!("event-{host}-{scope_distance}"),
            source_id: None,
            source_uuid: None,
            event_type: "DNS_NAME".to_owned(),
            data: json!(host),
            data_redacted: false,
            metadata: json!({}),
            metadata_redacted: false,
            host: Some(host.to_owned()),
            port: None,
            parent_id: None,
            parent_source_id: None,
            parent_source_uuid: None,
            timestamp: "2026-01-01T00:00:00Z".to_owned(),
            tags: tags.iter().map(|tag| (*tag).to_owned()).collect(),
            module: Some("archive".to_owned()),
            locator: host.to_owned(),
            depth: 0,
            scope_distance,
        }
    }

    #[test]
    fn aggregates_external_affiliates_without_promoting_scope() {
        let events = vec![
            event("api.example.com", 1, &["affiliate"]),
            event("a.partner.net", 2, &["affiliate"]),
            event("b.partner.net", 4, &[]),
            event("child.other.org", 1, &[]),
            event("excluded.net", 0, &["affiliate"]),
        ];
        let result = summarize_affiliate_domains(&events, &["example.com".to_owned()]).unwrap();

        assert_eq!(result.len(), 2);
        assert_eq!(result[0].domain, "partner.net");
        assert_eq!(result[0].score_milli, 1750);
        assert_eq!(result[0].event_count, 2);
        assert_eq!(result[1].domain, "other.org");
    }

    #[test]
    fn ignores_invalid_hosts_and_validates_explicit_roots() {
        let events = vec![event("192.0.2.4", 1, &[]), event("not_a_domain", 1, &[])];
        assert!(
            summarize_affiliate_domains(&events, &["example.com".to_owned()])
                .unwrap()
                .is_empty()
        );
        assert!(summarize_affiliate_domains(&events, &["https://example.com".to_owned()]).is_err());
    }

    #[test]
    fn enforces_event_and_scope_root_limits() {
        let events = vec![event("a.partner.net", 1, &[]); 10_001];
        assert!(summarize_affiliate_domains(&events, &["example.com".to_owned()]).is_err());
        let roots = (0..33)
            .map(|index| format!("scope{index}.example.com"))
            .collect::<Vec<_>>();
        assert!(summarize_affiliate_domains(&[], &roots).is_err());
    }
}
