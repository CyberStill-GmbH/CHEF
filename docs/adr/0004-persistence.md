# ADR 0004: Persistencia incremental

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

Vigencia: describe el incremento CLI offline implementado en TypeScript. La persistencia local de escritorio se decide en [ADR 0012](0012-escritorio-tauri-rust.md): SurrealDB embebida con RocksDB. El adaptador Rust ya guarda snapshot, eventos de origen saneados y adaptación CHEF tipada en el mismo documento; consultas de grafo y gates de producción siguen pendientes. Mantener Snapshot como formato de intercambio durante la migración.

## Contexto

Demo necesita exportar; persistencia completa todavía no es requisito del primer slice.

## Opciones

Snapshots JSON; para el escritorio, consultar la decisión SurrealDB de ADR 0012.

## Decisión

El incremento descrito aquí usa memoria + exportación JSON. El escritorio usa el puerto de repositorio con el adaptador SurrealDB decidido en ADR 0012.

## Consecuencias

JSON sencillo y auditable; no guardar datos originales por defecto. El esquema local SurrealDB debe versionarse y mantener referencias e integridad. No confundir formato de exportación con base de datos.

## Revisión prevista

Al iniciar historia de sesiones/multi-run.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
