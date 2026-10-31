//! Selección determinista de un código HTTP representativo para informes CHEF.
//!
//! Adapta `core/helpers/misc.py::best_http_status`. Se limita a códigos HTTP
//! numéricos ya archivados: no consulta destinos ni evalúa seguridad del recurso.

use crate::{ChefError, ChefResult, ErrorCode};

/// Elige el estado preferido según la regla de resumen de la fuente.
///
/// Orden de clases: 2xx, 3xx, 1xx, 4xx, 5xx; dentro de una clase gana el menor
/// código. Los valores fuera del rango HTTP 100–599 se rechazan como entrada
/// inválida CHEF.
pub fn select_preferred_http_status(left: u16, right: u16) -> ChefResult<u16> {
    if !(100..=599).contains(&left) || !(100..=599).contains(&right) {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Los códigos HTTP deben estar entre 100 y 599",
        ));
    }
    let left_priority = class_priority(left / 100);
    let right_priority = class_priority(right / 100);
    Ok(match left_priority.cmp(&right_priority) {
        std::cmp::Ordering::Less => left,
        std::cmp::Ordering::Greater => right,
        std::cmp::Ordering::Equal => left.min(right),
    })
}

fn class_priority(class: u16) -> u8 {
    match class {
        2 => 1,
        3 => 2,
        1 => 3,
        4 => 4,
        5 => 5,
        _ => unreachable!("el llamador valida el rango HTTP"),
    }
}

#[cfg(test)]
mod tests {
    use super::select_preferred_http_status;
    use crate::ErrorCode;

    #[test]
    fn ranks_status_classes_as_the_source_report_helper() {
        assert_eq!(select_preferred_http_status(200, 404).unwrap(), 200);
        assert_eq!(select_preferred_http_status(301, 302).unwrap(), 301);
        assert_eq!(select_preferred_http_status(100, 404).unwrap(), 100);
        assert_eq!(select_preferred_http_status(404, 500).unwrap(), 404);
    }

    #[test]
    fn rejects_values_outside_http_status_range() {
        assert_eq!(
            select_preferred_http_status(99, 200).unwrap_err().code,
            ErrorCode::InvalidInput
        );
        assert!(select_preferred_http_status(200, 600).is_err());
    }
}
