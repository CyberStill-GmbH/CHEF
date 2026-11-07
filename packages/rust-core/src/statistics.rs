//! Estadísticas deterministas derivadas del grafo de eventos BBOT.
//!
//! Adapta el resumen del módulo `internal.aggregate` y `scanner.stats` de BBOT
//! a importaciones CHEF: conserva recuentos por tipo y productor. No inventa
//! velocidad ni consumo de módulos, datos que un archivo de eventos no registra.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{BbotEventGraph, ChefError, ChefResult, ErrorCode};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EventTypeCount {
    pub event_type: String,
    pub count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModuleProducedCount {
    pub module: String,
    pub produced_total: usize,
    pub by_type: Vec<EventTypeCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BbotEventStatistics {
    pub total_events: usize,
    pub unique_event_types: usize,
    pub modules_run: usize,
    pub duplicates_suppressed: usize,
    /// Recuento descendente; los empates se ordenan por tipo.
    pub by_type: Vec<EventTypeCount>,
    /// Solo eventos atribuibles a un productor identificado.
    pub by_module: Vec<ModuleProducedCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ModuleRuntimeStatistics {
    pub module: String,
    pub produced_total: usize,
    pub produced_by_type: Vec<EventTypeCount>,
    pub consumed_total: usize,
    pub consumed_by_type: Vec<EventTypeCount>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct BbotRuntimeStatistics {
    pub modules_run: usize,
    pub by_module: Vec<ModuleRuntimeStatistics>,
}

#[derive(Debug, Default)]
pub(crate) struct RuntimeStatisticsBuilder {
    by_module: BTreeMap<String, ModuleRuntimeAccumulator>,
}

#[derive(Debug, Default)]
struct ModuleRuntimeAccumulator {
    produced: BTreeMap<String, usize>,
    consumed: BTreeMap<String, usize>,
}

impl RuntimeStatisticsBuilder {
    pub(crate) fn record_consumed(&mut self, module: &str, event_type: &str) {
        if module.starts_with('_') {
            return;
        }
        *self
            .by_module
            .entry(module.to_owned())
            .or_default()
            .consumed
            .entry(event_type.to_owned())
            .or_default() += 1;
    }

    pub(crate) fn record_produced(&mut self, module: &str, event_type: &str) {
        *self
            .by_module
            .entry(module.to_owned())
            .or_default()
            .produced
            .entry(event_type.to_owned())
            .or_default() += 1;
    }

    pub(crate) fn finish(self, modules_run: usize) -> BbotRuntimeStatistics {
        let mut by_module = self
            .by_module
            .into_iter()
            .filter(|(name, _)| name != "SEED")
            .map(|(module, counts)| {
                let produced_by_type = counts_by_name(counts.produced);
                let consumed_by_type = counts_by_name(counts.consumed);
                ModuleRuntimeStatistics {
                    module,
                    produced_total: produced_by_type.iter().map(|entry| entry.count).sum(),
                    consumed_total: consumed_by_type.iter().map(|entry| entry.count).sum(),
                    produced_by_type,
                    consumed_by_type,
                }
            })
            .collect::<Vec<_>>();
        by_module.sort_by(|left, right| {
            right
                .produced_total
                .cmp(&left.produced_total)
                .then_with(|| right.consumed_total.cmp(&left.consumed_total))
                .then_with(|| left.module.cmp(&right.module))
        });
        BbotRuntimeStatistics {
            modules_run,
            by_module,
        }
    }
}

/// Resume eventos importados o generados por el motor de eventos CHEF.
///
/// Como BBOT, atribuye un `URL` a su evento padre `URL_UNVERIFIED` cuando éste
/// fue producido por un módulo. A diferencia del reporte en vivo de BBOT, un
/// grafo importado no contiene cuántos eventos consumió cada módulo ni marcas
/// temporales de procesamiento; esos contadores se omiten en lugar de inferirlos.
pub fn summarize_bbot_events(graph: &BbotEventGraph) -> ChefResult<BbotEventStatistics> {
    crate::validate_bbot_event_graph(graph)?;

    let by_id = graph
        .events
        .iter()
        .map(|event| (event.id.as_str(), event))
        .collect::<std::collections::HashMap<_, _>>();
    let mut by_type = BTreeMap::<String, usize>::new();
    let mut by_module = BTreeMap::<String, BTreeMap<String, usize>>::new();

    for event in &graph.events {
        *by_type.entry(event.event_type.clone()).or_default() += 1;

        let attributed_module = event
            .parent_id
            .as_deref()
            .and_then(|parent_id| by_id.get(parent_id))
            .filter(|parent| {
                event.event_type == "URL"
                    && parent.event_type == "URL_UNVERIFIED"
                    && parent.module.is_some()
            })
            .and_then(|parent| parent.module.as_deref())
            .or(event.module.as_deref());

        if let Some(module) = attributed_module.filter(|module| *module != "SEED") {
            *by_module
                .entry(module.to_owned())
                .or_default()
                .entry(event.event_type.clone())
                .or_default() += 1;
        }
    }

    let event_types = counts_by_frequency(by_type);
    let mut modules = by_module
        .into_iter()
        .map(|(module, type_counts)| {
            let by_type = counts_by_name(type_counts);
            let produced_total = by_type.iter().map(|entry| entry.count).sum();
            ModuleProducedCount {
                module,
                produced_total,
                by_type,
            }
        })
        .collect::<Vec<_>>();

    modules.sort_by(|left, right| {
        right
            .produced_total
            .cmp(&left.produced_total)
            .then_with(|| left.module.cmp(&right.module))
    });

    if event_types.iter().any(|item| item.event_type.len() > 64)
        || modules.iter().any(|item| item.module.len() > 128)
        || serde_json::to_vec(&modules)
            .map_err(|_| ChefError::new(ErrorCode::InvalidInput, "Estadísticas CHEF inválidas"))?
            .len()
            > 256 * 1024
    {
        return Err(ChefError::new(
            ErrorCode::LimitExceeded,
            "Las estadísticas BBOT superan los límites CHEF",
        ));
    }

    let unique_event_types = event_types.len();
    Ok(BbotEventStatistics {
        total_events: graph.events.len(),
        unique_event_types,
        modules_run: graph.modules_run,
        duplicates_suppressed: graph.duplicates_suppressed,
        by_type: event_types,
        by_module: modules,
    })
}

fn counts_by_frequency(counts: BTreeMap<String, usize>) -> Vec<EventTypeCount> {
    let mut result = counts_by_name(counts);
    result.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.event_type.cmp(&right.event_type))
    });
    result
}

fn counts_by_name(counts: BTreeMap<String, usize>) -> Vec<EventTypeCount> {
    counts
        .into_iter()
        .map(|(event_type, count)| EventTypeCount { event_type, count })
        .collect()
}
