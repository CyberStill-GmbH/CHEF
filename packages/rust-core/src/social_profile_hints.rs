//! Extrae perfiles sociales referenciados por una URL archivada.
//!
//! Los enlaces son pistas contenidas en la entrada; no prueban quién controla
//! un perfil ni una relación con el host de origen. No se visita ningún sitio.

use crate::{ChefError, ChefResult, ErrorCode};
use url::Url;

const MAX_URL_BYTES: usize = 8 * 1024;
const MAX_PROFILE_CHARS: usize = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SocialPlatform {
    LinkedIn,
    Facebook,
    Twitter,
    GitHub,
    Instagram,
    YouTube,
    Bitbucket,
    GitLab,
    Discord,
    DockerHub,
    HuggingFace,
    Postman,
    Linktree,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchivedSocialProfileHint {
    pub platform: SocialPlatform,
    pub profile_name: String,
    pub profile_url: String,
}

/// Interpreta una URL archivada y devuelve un perfil solo para dominios
/// sociales admitidos y rutas con forma conocida.
pub fn parse_archived_social_profile(
    url_text: &str,
) -> ChefResult<Option<ArchivedSocialProfileHint>> {
    if url_text.len() > MAX_URL_BYTES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "La URL archivada supera el límite para pistas sociales",
        ));
    }
    if url_text.chars().any(char::is_control) {
        return Ok(None);
    }
    let Ok(url) = Url::parse(url_text.trim()) else {
        return Ok(None);
    };
    if !matches!(url.scheme(), "http" | "https")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.port().is_some()
    {
        return Ok(None);
    }
    let Some(host) = url.host_str() else {
        return Ok(None);
    };
    let path = url
        .path_segments()
        .map(|segments| segments.collect::<Vec<_>>())
        .unwrap_or_default();
    let Some((platform, canonical_host, profile)) = identify_profile(host, &path) else {
        return Ok(None);
    };
    if !valid_profile(platform, profile) {
        return Ok(None);
    }
    let profile_name = if platform == SocialPlatform::Discord {
        profile.to_owned()
    } else {
        profile.to_ascii_lowercase()
    };
    let route = match platform {
        SocialPlatform::LinkedIn => format!("/{}/{}", path[0], profile_name),
        SocialPlatform::YouTube => format!("/@{}", profile_name),
        SocialPlatform::DockerHub => format!("/{}/{}", path[0], profile_name),
        _ => format!("/{}", profile_name),
    };
    Ok(Some(ArchivedSocialProfileHint {
        platform,
        profile_name,
        profile_url: format!("https://{canonical_host}{route}"),
    }))
}

fn identify_profile<'a>(
    host: &str,
    path: &'a [&'a str],
) -> Option<(SocialPlatform, &'static str, &'a str)> {
    let first = *path.first()?;
    let (platform, canonical_host, profile) = match host {
        "linkedin.com" | "www.linkedin.com" if matches!(first, "in" | "company") => {
            (SocialPlatform::LinkedIn, "www.linkedin.com", *path.get(1)?)
        }
        "facebook.com" | "www.facebook.com" => {
            (SocialPlatform::Facebook, "www.facebook.com", first)
        }
        "twitter.com" | "www.twitter.com" => (SocialPlatform::Twitter, "twitter.com", first),
        "github.com" | "www.github.com" => (SocialPlatform::GitHub, "github.com", first),
        "instagram.com" | "www.instagram.com" => {
            (SocialPlatform::Instagram, "www.instagram.com", first)
        }
        "youtube.com" | "www.youtube.com" if first.starts_with('@') => (
            SocialPlatform::YouTube,
            "www.youtube.com",
            first.strip_prefix('@')?,
        ),
        "bitbucket.org" | "www.bitbucket.org" => {
            (SocialPlatform::Bitbucket, "bitbucket.org", first)
        }
        "gitlab.com" | "gitlab.org" | "www.gitlab.com" | "www.gitlab.org" => {
            (SocialPlatform::GitLab, "gitlab.com", first)
        }
        "discord.gg" => (SocialPlatform::Discord, "discord.gg", first),
        "hub.docker.com" => {
            if !matches!(first, "r" | "u") {
                return None;
            }
            (SocialPlatform::DockerHub, "hub.docker.com", *path.get(1)?)
        }
        "huggingface.co" | "www.huggingface.co" => {
            (SocialPlatform::HuggingFace, "huggingface.co", first)
        }
        "postman.com" | "www.postman.com" => (SocialPlatform::Postman, "www.postman.com", first),
        "linktr.ee" | "www.linktr.ee" => (SocialPlatform::Linktree, "linktr.ee", first),
        _ => return None,
    };
    Some((platform, canonical_host, profile))
}

fn valid_profile(platform: SocialPlatform, profile: &str) -> bool {
    let length = profile.chars().count();
    if length == 0
        || length > MAX_PROFILE_CHARS
        || profile.starts_with('.')
        || profile.ends_with('.')
    {
        return false;
    }
    let allowed =
        |character: char| character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.');
    if !profile.chars().all(allowed) || profile.contains("..") {
        return false;
    }
    match platform {
        SocialPlatform::Twitter => {
            length <= 15
                && profile
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        }
        SocialPlatform::YouTube => profile
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_'),
        SocialPlatform::Linktree => (3..=30).contains(&length),
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::{SocialPlatform, parse_archived_social_profile};
    use crate::ErrorCode;

    #[test]
    fn extracts_supported_archived_profiles_and_drops_tracking_data() {
        let result = parse_archived_social_profile(
            "http://www.linkedin.com/company/Example-Co/?utm_source=archive#section",
        )
        .unwrap()
        .unwrap();
        assert_eq!(result.platform, SocialPlatform::LinkedIn);
        assert_eq!(result.profile_name, "example-co");
        assert_eq!(
            result.profile_url,
            "https://www.linkedin.com/company/example-co"
        );

        let youtube = parse_archived_social_profile("https://youtube.com/@Chef_Channel/videos")
            .unwrap()
            .unwrap();
        assert_eq!(youtube.platform, SocialPlatform::YouTube);
        assert_eq!(youtube.profile_url, "https://www.youtube.com/@chef_channel");
    }

    #[test]
    fn rejects_lookalike_hosts_credentials_and_unrecognized_routes() {
        for value in [
            "https://linkedin.com.attacker.example/in/name",
            "https://user@github.com/name",
            "https://github.com.evil/name",
            "https://linkedin.com/feed/updates",
            "file:///github.com/profile",
        ] {
            assert!(
                parse_archived_social_profile(value).unwrap().is_none(),
                "{value}"
            );
        }
    }

    #[test]
    fn enforces_url_and_profile_limits() {
        assert_eq!(
            parse_archived_social_profile(&format!("https://github.com/{}", "x".repeat(8192)))
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        let long_profile = format!("https://github.com/{}", "x".repeat(129));
        assert!(
            parse_archived_social_profile(&long_profile)
                .unwrap()
                .is_none()
        );
    }
}
