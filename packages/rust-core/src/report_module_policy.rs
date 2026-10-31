//! Política de estadísticas para módulos de informe CHEF.
//!
//! Adapta `modules/report/base.py::_stats_exclude`: consumir evidencia para
//! construir un informe no debe inflar las métricas de módulos de análisis.

use std::collections::BTreeSet;

use crate::{ChefError, ChefResult, ErrorCode, ModuleRuntimeStatistics};

const MAX_MODULES: usize = 512;
const MAX_MODULE_NAME_BYTES: usize = 128;

/// Excluye del resumen de análisis los módulos identificados como informes.
///
/// La entrada estadística se conserva fuera de la vista filtrada; este helper
/// solo produce una selección para presentación y no borra evidencia ni datos.
pub fn filter_report_module_statistics(
    statistics: &[ModuleRuntimeStatistics],
    report_module_ids: &[String],
) -> ChefResult<Vec<ModuleRuntimeStatistics>> {
    if statistics.len() > MAX_MODULES || report_module_ids.len() > MAX_MODULES {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "El resumen supera el máximo CHEF de 512 módulos",
        ));
    }
    if report_module_ids
        .iter()
        .any(|module| module.is_empty() || module.len() > MAX_MODULE_NAME_BYTES)
    {
        return Err(ChefError::new(
            ErrorCode::InvalidInput,
            "Un identificador de módulo de informe está vacío o es demasiado largo",
        ));
    }

    let report_modules = report_module_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    Ok(statistics
        .iter()
        .filter(|statistic| !report_modules.contains(statistic.module.as_str()))
        .cloned()
        .collect())
}

#[cfg(test)]
mod tests {
    use super::filter_report_module_statistics;
    use crate::{ErrorCode, EventTypeCount, ModuleRuntimeStatistics};

    fn statistics(module: &str, produced_total: usize) -> ModuleRuntimeStatistics {
        ModuleRuntimeStatistics {
            module: module.to_owned(),
            produced_total,
            produced_by_type: vec![EventTypeCount {
                event_type: "DNS_NAME".to_owned(),
                count: produced_total,
            }],
            consumed_total: produced_total,
            consumed_by_type: vec![EventTypeCount {
                event_type: "DNS_NAME".to_owned(),
                count: produced_total,
            }],
        }
    }

    #[test]
    fn excludes_report_modules_and_preserves_analysis_statistics() {
        let input = vec![statistics("dns_parser", 4), statistics("summary_report", 9)];
        let reports = vec!["summary_report".to_owned()];
        let filtered = filter_report_module_statistics(&input, &reports).unwrap();

        assert_eq!(filtered, vec![statistics("dns_parser", 4)]);
        assert_eq!(input.len(), 2);
    }

    #[test]
    fn validates_counts_and_report_identifiers() {
        let too_many = vec![statistics("module", 1); 513];
        assert_eq!(
            filter_report_module_statistics(&too_many, &[])
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
        assert_eq!(
            filter_report_module_statistics(&[], &[String::new()])
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
        assert!(filter_report_module_statistics(&[], &["x".repeat(129)]).is_err());
    }
}
