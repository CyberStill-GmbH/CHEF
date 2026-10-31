//! Catálogo de módulos CHEF que pueden habilitarse desde una configuración.
//!
//! El registro es estático: una clave TOML no instala código, no carga plugins
//! y no convierte una capacidad propuesta en ejecutable.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleCapabilityState {
    Implemented,
    Proposed,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleCapabilityKind {
    EventTransform,
    ImportTransform,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleCapability {
    pub id: &'static str,
    pub kind: ModuleCapabilityKind,
    pub state: ModuleCapabilityState,
    pub offline_only: bool,
}

const CATALOG: &[ModuleCapability] = &[
    ModuleCapability {
        id: "chef_bimi_url_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_caa_record_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_cloud_storage_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_common_port_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_dns_mutation_hints",
        kind: ModuleCapabilityKind::ImportTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_dns_parent_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_dns_rdata_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_ip_range_expansion",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_org_stub_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_tlsrpt_target_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_url_attribute_hints",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
    ModuleCapability {
        id: "chef_url_parents",
        kind: ModuleCapabilityKind::EventTransform,
        state: ModuleCapabilityState::Implemented,
        offline_only: true,
    },
];

pub fn module_capabilities() -> &'static [ModuleCapability] {
    CATALOG
}

pub fn module_capability(id: &str) -> Option<&'static ModuleCapability> {
    CATALOG.iter().find(|capability| capability.id == id)
}
