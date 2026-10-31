//! Extrae pistas de rutas desde contenido `robots.txt` archivado.
//!
//! El parser no recupera el archivo, no prueba las rutas ni las convierte en
//! observaciones actuales. Los valores se conservan como referencias históricas.

use std::collections::BTreeSet;

use url::Url;

use crate::{ChefError, ChefResult, ErrorCode};

const MAX_ROBOTS_BYTES: usize = 1024 * 1024;
const MAX_HINTS: usize = 4096;
const MAX_VALUE_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RobotsHintKind {
    Permitir,
    Restringir,
    MapaDelSitio,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotsPathHint {
    pub kind: RobotsHintKind,
    /// Valor literal de robots.txt, incluidos los comodines si los hubiera.
    pub value: String,
    /// URL del mismo origen cuando el valor es concreto y seguro de resolver.
    /// Los patrones con comodines no se transforman en URL.
    pub url_hint: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedRobotsHints {
    pub origin: String,
    pub hints: Vec<RobotsPathHint>,
}

/// Interpreta directivas Allow, Disallow y Sitemap de un texto archivado.
///
/// Solo acepta rutas del mismo origen suministrado. Las directivas se devuelven
/// como pistas; en particular, `Disallow` no significa que una ruta exista.
pub fn parse_archived_robots_hints(
    content: &str,
    base_origin: &str,
) -> ChefResult<ArchivedRobotsHints> {
    if content.len() > MAX_ROBOTS_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El archivo robots archivado supera el límite de tamaño",
        ));
    }
    let base = parse_origin(base_origin)?;
    let origin = base.origin().ascii_serialization();
    let mut hints = Vec::new();
    let mut directives_seen = BTreeSet::new();

    for line in content.lines() {
        let line = line
            .split_once('#')
            .map_or(line, |(before, _)| before)
            .trim();
        let Some((name, raw_value)) = line.split_once(':') else {
            continue;
        };
        let kind = match name.trim().to_ascii_lowercase().as_str() {
            "allow" => RobotsHintKind::Permitir,
            "disallow" => RobotsHintKind::Restringir,
            "sitemap" => RobotsHintKind::MapaDelSitio,
            _ => continue,
        };
        let raw_value = raw_value.trim();
        if raw_value.is_empty()
            || raw_value.len() > MAX_VALUE_BYTES
            || raw_value.chars().any(char::is_control)
        {
            continue;
        }
        let suffix_start = raw_value
            .find('?')
            .into_iter()
            .chain(raw_value.find('#'))
            .min()
            .unwrap_or(raw_value.len());
        let value = raw_value[..suffix_start].trim_end();
        if value.is_empty() {
            continue;
        }
        let value = value.to_owned();
        if !directives_seen.insert((kind, value.clone())) {
            continue;
        }
        let Some(url_hint) = resolve_same_origin_hint(&base, kind, &value) else {
            continue;
        };
        if hints.len() == MAX_HINTS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El archivo robots archivado supera el máximo de pistas",
            ));
        }
        hints.push(RobotsPathHint {
            kind,
            value,
            url_hint,
        });
    }

    Ok(ArchivedRobotsHints { origin, hints })
}

fn parse_origin(value: &str) -> ChefResult<Url> {
    let url = Url::parse(value).map_err(|_| {
        ChefError::new(
            ErrorCode::InvalidInput,
            "El origen robots no es una URL válida",
        )
    })?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "El origen robots debe ser HTTP(S) sin credenciales, query ni fragmento",
        ));
    }
    Ok(url)
}

fn resolve_same_origin_hint(
    base: &Url,
    kind: RobotsHintKind,
    value: &str,
) -> Option<Option<String>> {
    if value.contains('\\') || value.chars().any(char::is_whitespace) {
        return None;
    }

    let mut resolved = match kind {
        RobotsHintKind::MapaDelSitio => Url::parse(value).or_else(|_| base.join(value)).ok()?,
        RobotsHintKind::Permitir | RobotsHintKind::Restringir => {
            if !value.starts_with('/') || value.starts_with("//") {
                return None;
            }
            base.join(value).ok()?
        }
    };
    if resolved.origin() != base.origin()
        || resolved.username() != base.username()
        || resolved.password() != base.password()
        || !matches!(resolved.scheme(), "http" | "https")
    {
        return None;
    }
    resolved.set_query(None);
    resolved.set_fragment(None);

    // Los comodines de robots no identifican una URL concreta. Se conserva el
    // patrón literal, pero no se publica como candidato de navegación.
    let concrete_url = (!value.contains('*') && !value.contains('$')).then(|| resolved.to_string());
    Some(concrete_url)
}

#[cfg(test)]
mod tests {
    use super::{RobotsHintKind, parse_archived_robots_hints, resolve_same_origin_hint};
    use crate::ErrorCode;
    use url::Url;

    #[test]
    fn parses_archived_directives_as_same_origin_hints() {
        let text = "User-agent: *\nDisallow: /private # archived\nAllow: /assets\nDisallow: /tmp/*\nSitemap: https://example.com/sitemap.xml\n";
        let result = parse_archived_robots_hints(text, "https://example.com").unwrap();

        assert_eq!(result.origin, "https://example.com");
        assert_eq!(result.hints.len(), 4);
        assert_eq!(result.hints[0].kind, RobotsHintKind::Restringir);
        assert_eq!(
            result.hints[0].url_hint.as_deref(),
            Some("https://example.com/private")
        );
        assert_eq!(result.hints[2].value, "/tmp/*");
        assert_eq!(result.hints[2].url_hint, None);
        assert_eq!(result.hints[3].kind, RobotsHintKind::MapaDelSitio);
    }

    #[test]
    fn rejects_foreign_origins_and_non_path_directives() {
        let text = "Disallow: //evil.example/path\nAllow: https://evil.example/x\nSitemap: https://other.example/map.xml\nUnknown: /ignored\nDisallow: /ok\n";
        let result = parse_archived_robots_hints(text, "https://example.com").unwrap();

        assert_eq!(result.hints.len(), 1);
        assert_eq!(result.hints[0].value, "/ok");
    }

    #[test]
    fn strips_query_and_fragment_from_hints_and_deduplicates_identical_directives() {
        let text = "Allow: /assets?token=private\nAllow: /assets?token=private\nSitemap: https://example.com/map.xml?key=private\n";
        let result = parse_archived_robots_hints(text, "https://example.com").unwrap();

        assert_eq!(result.hints.len(), 2);
        assert_eq!(result.hints[0].value, "/assets");
        assert_eq!(
            result.hints[0].url_hint.as_deref(),
            Some("https://example.com/assets")
        );
        assert_eq!(
            result.hints[1].url_hint.as_deref(),
            Some("https://example.com/map.xml")
        );
        assert!(result.hints.iter().all(|hint| {
            !hint.value.contains("key=private")
                && !hint.value.contains("token=private")
                && !hint.value.contains('?')
                && !hint.value.contains('#')
                && hint
                    .url_hint
                    .as_deref()
                    .is_none_or(|url| !url.contains('?') && !url.contains('#'))
        }));
    }

    #[test]
    fn strips_query_and_fragment_from_resolved_sitemap_url() {
        let base = Url::parse("https://example.com").unwrap();
        assert_eq!(
            resolve_same_origin_hint(
                &base,
                RobotsHintKind::MapaDelSitio,
                "https://example.com/map.xml?key=private#section",
            ),
            Some(Some("https://example.com/map.xml".to_owned()))
        );
    }

    #[test]
    fn enforces_size_count_and_origin_limits() {
        let oversized = "x".repeat(1024 * 1024 + 1);
        assert_eq!(
            parse_archived_robots_hints(&oversized, "https://example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );

        let too_many = (0..4097)
            .map(|index| format!("Allow: /{index}\n"))
            .collect::<String>();
        assert_eq!(
            parse_archived_robots_hints(&too_many, "https://example.com")
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            parse_archived_robots_hints("", "file:///tmp/robots.txt")
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
