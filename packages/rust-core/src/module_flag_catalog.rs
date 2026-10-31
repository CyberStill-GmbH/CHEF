//! Etiquetas legibles para clasificar capacidades CHEF en catálogos locales.
//!
//! El texto de una etiqueta es descriptivo: no autoriza conexiones, ejecución
//! de módulos ni ampliación de alcance.

const MAX_FLAG_BYTES: usize = 64;

/// Devuelve una descripción en español para una etiqueta de capacidad conocida.
///
/// `None` significa que la etiqueta no pertenece al vocabulario CHEF reconocido.
/// La entrada se limita para que pueda provenir de un catálogo importado.
pub fn describe_module_flag(flag: &str) -> Option<&'static str> {
    if flag.is_empty() || flag.len() > MAX_FLAG_BYTES {
        return None;
    }
    let description = match flag {
        "active" => {
            "Capacidad de conexión activa; requiere autorización explícita y permanece desactivada por defecto."
        }
        "affiliates" => "Contexto de dominios relacionados; no acredita afiliación ni propiedad.",
        "baddns" => "Clasificación de auditoría DNS; no habilita consultas ni pruebas activas.",
        "cloud-enum" => {
            "Contexto de recursos cloud; no acredita propiedad ni disponibilidad actual."
        }
        "code-enum" => {
            "Contexto de repositorios públicos; no implica extracción de secretos ni consultas remotas."
        }
        "download" => "Capacidad de descarga; no se ejecuta desde evidencia importada.",
        "email-enum" => {
            "Clasificación de candidatos de correo; no confirma que una dirección exista."
        }
        "iis-shortnames" => "Contexto de nombres cortos IIS; no declara vulnerabilidad confirmada.",
        "invasive" => "Capacidad intrusiva; bloqueada en el flujo offline CHEF.",
        "loud" => "Capacidad potencialmente ruidosa; no habilita tráfico de red.",
        "passive" => "Contexto pasivo archivado; no representa una observación actual.",
        "safe" => {
            "Etiqueta declarativa de impacto; no sustituye la revisión de la operación concreta."
        }
        "portscan" => {
            "Capacidad de descubrimiento de puertos; requiere autorización y permanece desactivada."
        }
        "report" => "Generación local de informes desde evidencia ya disponible.",
        "service-enum" => "Clasificación de servicios; no realiza conexiones para identificarlos.",
        "slow" => "Indica una operación potencialmente larga; no configura ejecución CHEF.",
        "social-enum" => {
            "Contexto de perfiles sociales; no implica consulta remota ni atribución de identidad."
        }
        "subdomain-enum" => {
            "Contexto de candidatos de subdominio; no acredita resolución DNS actual."
        }
        "subdomain-hijack" => {
            "Contexto de riesgo de control DNS; no declara toma de control confirmada."
        }
        "web" => "Contexto de evidencia web archivada; no inicia solicitudes HTTP.",
        "web-heavy" => "Capacidad web avanzada; no se ejecuta en el flujo offline CHEF.",
        "web-paramminer" => {
            "Capacidad de búsqueda de parámetros; no genera solicitudes ni fuerza bruta."
        }
        "web-screenshots" => "Capacidad de captura visual; no carga páginas ni guarda archivos.",
        _ => return None,
    };
    Some(description)
}

#[cfg(test)]
mod tests {
    use super::{MAX_FLAG_BYTES, describe_module_flag};

    #[test]
    fn describes_report_and_network_capability_labels_without_authorizing_them() {
        assert!(describe_module_flag("report").unwrap().contains("informes"));
        assert!(
            describe_module_flag("active")
                .unwrap()
                .contains("desactivada")
        );
        assert!(
            describe_module_flag("passive")
                .unwrap()
                .contains("archivado")
        );
    }

    #[test]
    fn rejects_unknown_empty_and_overlong_labels() {
        assert_eq!(describe_module_flag("unknown"), None);
        assert_eq!(describe_module_flag(""), None);
        assert_eq!(describe_module_flag(&"x".repeat(MAX_FLAG_BYTES + 1)), None);
    }
}
