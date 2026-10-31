//! Adapta páginas archivadas de tareas de SubdomainRadar a pistas CHEF.
//!
//! El parser no crea tareas ni sondea o resuelve los nombres devueltos.

use std::collections::BTreeSet;

use serde_json::Value;

use crate::{ChefError, ChefResult, ErrorCode, host::canonical_dns_name};

const MAX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_SUBDOMAIN_ENTRIES: usize = 4096;
const MAX_CANDIDATES: usize = 512;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSubdomainRadarPage {
    pub query_domain: String,
    pub completed: bool,
    pub candidates: Vec<String>,
}

/// Interpreta el estado y `subdomains[].subdomain` de una respuesta archivada.
///
/// Solo las páginas completadas producen candidatos, estrictamente bajo la
/// raíz consultada y distintos del nombre de evento que inició la tarea.
pub fn parse_archived_subdomainradar_page(
    query_domain: &str,
    excluded_domain: &str,
    response: &[u8],
) -> ChefResult<ArchivedSubdomainRadarPage> {
    if response.len() > MAX_RESPONSE_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La respuesta de SubdomainRadar archivada supera el límite de bytes",
        ));
    }
    let query_domain = canonical_dns_name(query_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La consulta SubdomainRadar debe ser un nombre DNS válido",
    ))?;
    if !crate::host::is_registrable_domain(&query_domain) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "La consulta SubdomainRadar debe ser una raíz registrable ICANN",
        ));
    }
    let excluded_domain = canonical_dns_name(excluded_domain).ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "El dominio de evento excluido no es válido",
    ))?;
    let document: Value = serde_json::from_slice(response).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "La respuesta SubdomainRadar no contiene JSON válido",
        )
    })?;
    let object = document.as_object().ok_or(ChefError::new(
        ErrorCode::InvalidInput,
        "La respuesta SubdomainRadar debe ser un objeto JSON",
    ))?;
    let completed = object
        .get("status")
        .and_then(Value::as_str)
        .is_some_and(|status| status.eq_ignore_ascii_case("completed"));
    let mut candidates = BTreeSet::new();
    if completed {
        let subdomains =
            object
                .get("subdomains")
                .and_then(Value::as_array)
                .ok_or(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Una tarea completada debe incluir una lista subdomains",
                ))?;
        if subdomains.len() > MAX_SUBDOMAIN_ENTRIES {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "La respuesta supera el máximo de entradas subdomain",
            ));
        }
        let suffix = format!(".{query_domain}");
        for value in subdomains {
            let Some(value) = value
                .as_object()
                .and_then(|object| object.get("subdomain"))
                .and_then(Value::as_str)
            else {
                continue;
            };
            let Some(hostname) = canonical_dns_name(value) else {
                continue;
            };
            if hostname != excluded_domain && hostname.ends_with(&suffix) {
                candidates.insert(hostname);
                if candidates.len() > MAX_CANDIDATES {
                    return Err(ChefError::new(
                        ErrorCode::LimitExceeded,
                        "La respuesta supera el máximo CHEF de candidatos DNS",
                    ));
                }
            }
        }
    }
    Ok(ArchivedSubdomainRadarPage {
        query_domain,
        completed,
        candidates: candidates.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::parse_archived_subdomainradar_page;
    use crate::ErrorCode;

    #[test]
    fn extracts_only_completed_in_scope_task_results() {
        let response = br#"{"status":"completed","subdomains":[{"subdomain":"api.example.com"},{"subdomain":"event.example.com"},{"subdomain":"evil-example.com"},{"subdomain":"API.EXAMPLE.COM"},{}]}"#;
        let result =
            parse_archived_subdomainradar_page("EXAMPLE.COM.", "event.example.com", response)
                .unwrap();
        assert!(result.completed);
        assert_eq!(result.candidates, vec!["api.example.com"]);
    }

    #[test]
    fn ignores_pending_status_and_rejects_bad_shapes() {
        let pending = parse_archived_subdomainradar_page(
            "example.com",
            "event.example.com",
            br#"{"status":"running"}"#,
        )
        .unwrap();
        assert!(!pending.completed);
        assert!(pending.candidates.is_empty());
        assert_eq!(
            parse_archived_subdomainradar_page(
                "example.com",
                "event.example.com",
                br#"{"status":"completed","subdomains":{}}"#
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }

    #[test]
    fn enforces_response_and_entry_limits() {
        let oversized = vec![b'x'; 1024 * 1024 + 1];
        assert_eq!(
            parse_archived_subdomainradar_page("example.com", "event.example.com", &oversized)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let entries = vec![serde_json::json!({"subdomain":"a.example.com"}); 4097];
        let response =
            serde_json::to_vec(&serde_json::json!({"status":"completed","subdomains":entries}))
                .unwrap();
        assert_eq!(
            parse_archived_subdomainradar_page("example.com", "event.example.com", &response)
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }
}
