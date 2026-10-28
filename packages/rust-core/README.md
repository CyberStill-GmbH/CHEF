# Núcleo CHEF en Rust

Este crate implementa el primer corte del núcleo Rust CHEF. Procesa Nmap y BBOT JSONL entregados como bytes y produce `Snapshot 1.0.0` o `1.1.0`; `import_censys_ip_snapshot` convierte además una respuesta Censys Hosts archivada en un grafo de eventos con hash, IDs deterministas y linaje. El parser conserva los eventos, sus datos, UUID y metadatos. `recon.rs` proyecta el grafo interno CHEF 1.4.0 (lee 1.0.0–1.3.0) a activos de dominio, IP, servicio y endpoint web con evidencia; para Censys, esas relaciones quedan calificadas como `hint`. `TECHNOLOGY_HINT` y `PROTOCOL_HINT` se preservan como tipos sin proyección hasta una decisión de contrato posterior. `DNS_NAME_UNRESOLVED` permanece marcado sin resolver; las derivaciones de dominio/IP/organización se marcan como pistas y una observación directa prevalece al fusionar. `FINDING` nunca se presenta como vulnerabilidad confirmada. Los puertos sugeridos se distinguen de los puertos abiertos importados. Los tipos aún no adaptados quedan contabilizados y los eventos fuente persisten para ampliar la cobertura. `event_engine.rs` aporta despacho recursivo de módulos compilados, deduplicación y límites. No ofrece filesystem, shell ni acceso de red.

La coordinación Tokio de importaciones y el lock por clave viven en `packages/rust-application`, fuera de este núcleo. `chef-core` se limita a reglas y modelos sin runtime async. El núcleo TypeScript se conserva durante la migración como referencia ejecutable. La cobertura de pruebas del port Rust se realizará en una fase posterior, como indicó el propietario.

`config.rs` porta la configuración al modelo TOML de CHEF: admite límites, scope literal, módulos compilados y fuentes desactivadas por defecto; rechaza claves de API y no permite activar descubrimiento desde el archivo. El ejemplo está en [`config.example.toml`](config.example.toml).

Desde este directorio:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

El importador de Censys todavía no está compuesto en Tauri ni persiste sus resultados por `packages/rust-storage`; tampoco ejecuta escaneo activo o consultas pasivas. La persistencia local vive en `packages/rust-storage`, que guarda el grafo de eventos BBOT y los activos, evidencias y relaciones CHEF en el mismo registro SurrealDB. Aún faltan las implementaciones funcionales de módulos BBOT, presets, resolutores, CLI y salidas adaptadas. No se declara migración completa.
