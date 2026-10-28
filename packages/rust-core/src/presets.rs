//! Perfiles CHEF que adaptan la composición de presets de BBOT a flujos
//! explícitos del producto. No cargan plugins ni activan conectores/red.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{ChefConfig, ChefError, ChefResult, ErrorCode};

const MAX_PRESET_DEPTH: usize = 8;
const MAX_PRESETS: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ChefPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub includes: Vec<String>,
    #[serde(default)]
    pub modules: BTreeMap<String, bool>,
    #[serde(default)]
    pub sources: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub modules: BTreeMap<String, bool>,
    pub sources: BTreeMap<String, bool>,
}

#[derive(Debug, Clone, Default)]
pub struct PresetCatalog {
    presets: BTreeMap<String, ChefPreset>,
}

impl PresetCatalog {
    /// Registra perfiles CHEF declarativos. Los perfiles incluidos describen
    /// operaciones locales que ya existen, no módulos de red aún pendientes.
    pub fn new(presets: impl IntoIterator<Item = ChefPreset>) -> ChefResult<Self> {
        let mut catalog = Self::default();
        for preset in presets {
            if !valid_name(&preset.id)
                || preset.name.is_empty()
                || preset.name.len() > 128
                || preset.description.len() > 512
                || preset.includes.len() > MAX_PRESETS
                || preset.modules.len() > 256
                || preset.sources.len() > 128
                || preset
                    .includes
                    .iter()
                    .chain(preset.modules.keys())
                    .chain(preset.sources.keys())
                    .any(|name| !valid_name(name))
                || catalog.presets.insert(preset.id.clone(), preset).is_some()
            {
                return Err(ChefError::new(
                    ErrorCode::InvalidInput,
                    "Definición de preset CHEF inválida o duplicada",
                ));
            }
        }
        if catalog.presets.len() > MAX_PRESETS {
            return Err(ChefError::new(
                ErrorCode::LimitExceeded,
                "El catálogo de presets CHEF supera el máximo",
            ));
        }
        Ok(catalog)
    }

    pub fn resolve(&self, id: &str) -> ChefResult<ResolvedPreset> {
        if !valid_name(id) {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "Identificador de preset CHEF inválido",
            ));
        }
        let mut stack = BTreeSet::new();
        let mut modules = BTreeMap::new();
        let mut sources = BTreeMap::new();
        let mut resolved = self.resolve_into(id, 0, &mut stack, &mut modules, &mut sources)?;
        resolved.modules = modules;
        resolved.sources = sources;
        Ok(resolved)
    }

    pub fn built_in() -> Self {
        // Constantes internas: esta construcción no puede fallar.
        Self::new([
            ChefPreset {
                id: "offline_import".to_owned(),
                name: "Importación local".to_owned(),
                description: "Importa fixtures Nmap o JSONL BBOT y prepara el grafo de evidencia CHEF sin red.".to_owned(),
                includes: Vec::new(),
                modules: BTreeMap::from([
                    ("nmap_import".to_owned(), true),
                    ("bbot_jsonl_import".to_owned(), true),
                    ("recon_projection".to_owned(), true),
                ]),
                sources: BTreeMap::new(),
            },
            ChefPreset {
                id: "evidence_review".to_owned(),
                name: "Revisión de evidencia".to_owned(),
                description: "Abre evidencia importada, conserva procedencia y separa observaciones de candidatos.".to_owned(),
                includes: vec!["offline_import".to_owned()],
                modules: BTreeMap::from([("candidate_review".to_owned(), true)]),
                sources: BTreeMap::new(),
            },
            ChefPreset {
                id: "offline_hints".to_owned(),
                name: "Importación con pistas offline".to_owned(),
                description: "Además de importar evidencia, sugiere nombres DNS, puertos TCP, organizaciones, almacenamiento en la nube y dominios padre con fuentes locales; expande rangos CIDR con límite, sin consultas ni conexiones.".to_owned(),
                includes: vec!["offline_import".to_owned()],
                modules: BTreeMap::from([
                    ("chef_ip_range_expansion".to_owned(), true),
                    ("chef_dns_parent_hints".to_owned(), true),
                    ("chef_dns_rdata_hints".to_owned(), true),
                    ("chef_dns_mutation_hints".to_owned(), true),
                    ("chef_common_port_hints".to_owned(), true),
                    ("chef_cloud_storage_hints".to_owned(), true),
                    ("chef_org_stub_hints".to_owned(), true),
                    ("chef_url_attribute_hints".to_owned(), true),
                    ("chef_url_parents".to_owned(), true),
                ]),
                sources: BTreeMap::new(),
            },
        ])
        .expect("presets CHEF integrados válidos")
    }

    fn resolve_into(
        &self,
        id: &str,
        depth: usize,
        stack: &mut BTreeSet<String>,
        modules: &mut BTreeMap<String, bool>,
        sources: &mut BTreeMap<String, bool>,
    ) -> ChefResult<ResolvedPreset> {
        if depth >= MAX_PRESET_DEPTH || !stack.insert(id.to_owned()) {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "La composición de presets CHEF contiene ciclo o profundidad excesiva",
            ));
        }
        let Some(preset) = self.presets.get(id) else {
            return Err(ChefError::new(
                ErrorCode::InvalidInput,
                "El preset CHEF incluido no existe",
            ));
        };
        for included in &preset.includes {
            self.resolve_into(included, depth + 1, stack, modules, sources)?;
        }
        modules.extend(preset.modules.clone());
        sources.extend(preset.sources.clone());
        stack.remove(id);
        Ok(ResolvedPreset {
            id: preset.id.clone(),
            name: preset.name.clone(),
            description: preset.description.clone(),
            modules: BTreeMap::new(),
            sources: BTreeMap::new(),
        })
    }
}

impl ResolvedPreset {
    /// Aplica al config solamente capacidades explícitas del preset. CHEF
    /// continúa bloqueando descubrimiento activo en `ChefConfig::validate`.
    pub fn apply(&self, config: &mut ChefConfig) -> ChefResult<()> {
        config.modules.extend(self.modules.clone());
        config.sources.extend(
            self.sources
                .iter()
                .map(|(name, enabled)| (name.clone(), crate::SourceSettings { enabled: *enabled })),
        );
        config.validate()
    }
}

fn valid_name(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_-".contains(&byte))
}
