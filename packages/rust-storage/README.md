# Persistencia local CHEF

Adaptador Rust para guardar/recuperar snapshots CHEF `1.0.0` y `1.1.0` en SurrealDB embebida con RocksDB, bajo su `runId` determinista. Para BBOT guarda en un único upsert el snapshot, los eventos fuente completos y el grafo CHEF 1.4.0 (también lee 1.0.0–1.3.0): activos tipados, evidencia, relaciones observadas o sugeridas y conteo de tipos aún no adaptados. Depende de `chef-core`, no al revés; la composición de escritorio entrega la ruta de datos desde Tauri.

Esta es una integración inicial de documento versionado con clave determinista; cada guardado reemplaza un solo registro completo y el motor mantiene la atomicidad del registro. Las relaciones CHEF se guardan con semántica tipada; todavía faltan consultas de producto e interfaz Tauri. El indexado vectorial se decidirá solo si las búsquedas reales lo requieren. La prueba usa un directorio temporal, cierra el primer handle y verifica la reapertura sin red.

```powershell
cargo test --manifest-path packages/rust-storage/Cargo.toml --locked
```

SurrealDB queda fijada en `Cargo.toml` y `Cargo.lock`; el motor RocksDB está embebido y persiste en un directorio, no requiere un servidor aparte. El spike de producción debe evaluar migraciones, backup/restauración, recuperación tras cierre inesperado, bloqueo del directorio, tamaño de instalador y compatibilidad en cada sistema operativo. En Windows el build requiere NASM y toolchain nativo; el entorno actual no los tiene, por lo que el crate aún no supera una prueba de compilación local.
