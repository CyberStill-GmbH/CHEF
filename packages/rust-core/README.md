# Núcleo CHEF en Rust

Este crate implementa el primer corte del núcleo Rust CHEF. Procesa Nmap y BBOT JSONL entregados como bytes y produce `Snapshot 1.0.0` o `1.1.0`. El parser conserva los eventos BBOT, sus datos, UUID, metadatos y linaje. `recon.rs` proyecta el grafo interno CHEF 1.4.0 (lee 1.0.0–1.3.0) a activos de dominio, IP, servicio, endpoint web, tecnología, certificado, organización sugerida y hallazgo candidato, con evidencia. `DNS_NAME_UNRESOLVED` permanece marcado sin resolver; las derivaciones de dominio/IP/organización se marcan como pistas y una observación directa prevalece al fusionar. `FINDING` nunca se presenta como vulnerabilidad confirmada. Los puertos TCP derivados se guardan como `TCP_PORT_HINT`, distintos de `OPEN_TCP_PORT`. Los tipos aún no adaptados quedan contabilizados y los eventos fuente persisten para ampliar la cobertura. `event_engine.rs` aporta despacho recursivo de módulos compilados, deduplicación y límites. No ofrece filesystem, shell ni acceso de red.

`jobs.rs` coordina importaciones con Tokio, ejecuta el trabajo bloqueante fuera del executor asíncrono, limita el canal de estados y permite cancelación cooperativa. El núcleo TypeScript se conserva durante la migración como referencia ejecutable. La cobertura de pruebas del port Rust se realizará en una fase posterior, como indicó el propietario.

`config.rs` porta la configuración al modelo TOML de CHEF: admite límites, scope literal, módulos compilados y fuentes desactivadas por defecto; rechaza claves de API y no permite activar descubrimiento desde el archivo. El ejemplo está en [`config.example.toml`](config.example.toml).

Desde este directorio:

```powershell
cargo fmt --check
cargo test
cargo clippy --all-targets -- -D warnings
```

Este crate todavía no importa snapshots externos ni compone Tauri; tampoco accede directamente al almacenamiento ni ejecuta escaneo activo/consultas pasivas. La persistencia local vive en `packages/rust-storage`, que guarda el grafo de eventos original y los activos, evidencias y relaciones CHEF en el mismo registro SurrealDB. Aún faltan las implementaciones funcionales de módulos BBOT, presets, resolutores, CLI y salidas adaptadas. No se declara migración completa.
