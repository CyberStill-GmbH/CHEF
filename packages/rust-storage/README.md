# Persistencia local CHEF

Adaptador Rust para guardar/recuperar snapshots CHEF `1.0.0` y `1.1.0` en SurrealDB embebida con RocksDB, bajo su `runId` determinista. Para las importaciones JSONL heredadas guarda en un único upsert el snapshot, los eventos fuente completos y el grafo CHEF 1.4.0 (también lee 1.0.0–1.3.0): activos tipados, evidencia, relaciones observadas o sugeridas y conteo de tipos aún no adaptados. Las respuestas archivadas de Censys se guardan aparte en `provider_imports`, con esquema `censys-ip-import 1.0.0`, ID determinista, hash de respuesta, eventos fuente y grafo CHEF. Esta separación conserva el significado de Snapshot, que representa observaciones de servicios y no pistas históricas del proveedor. Depende de `chef-core`, no al revés; la composición de escritorio entrega la ruta de datos desde Tauri.

Cada importación se valida antes de persistir: IP canónica, hash SHA-256, IDs, grafos y proyección determinista. La lectura vuelve a verificar la identidad y versión del registro. Los datos se guardan localmente; este flujo no consulta Censys ni hace DNS/HTTP. El contrato del resumen IPC tiene [esquema](../../docs/contracts/censys-ip-import.schema.json) y [ejemplo](../../docs/contracts/examples/censys-ip-import.json); el chequeo documental valida que ambos concuerden. Las pruebas funcionales de almacenamiento e interfaz siguen aplazadas por el propietario. El indexado vectorial se decidirá solo si las búsquedas reales lo requieren.

```powershell
cargo test --manifest-path packages/rust-storage/Cargo.toml --locked
```

SurrealDB queda fijada en `Cargo.toml` y `Cargo.lock`; el motor RocksDB está embebido y persiste en un directorio, no requiere un servidor aparte. El spike de producción debe evaluar migraciones, backup/restauración, recuperación tras cierre inesperado, bloqueo del directorio, tamaño de instalador y compatibilidad en cada sistema operativo. En Windows el build de RocksDB requiere NASM, CMake, toolchain nativo y libclang; el entorno CHEF ya tiene estas herramientas instaladas en el perfil local, pero cada CI/estación de desarrollo debe documentar su propia configuración.
