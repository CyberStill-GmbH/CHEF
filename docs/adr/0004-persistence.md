# ADR 0004: Persistencia incremental

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

## Contexto

Demo necesita exportar; persistencia completa todavía no es requisito del primer slice.

## Opciones

JSON snapshots, SQLite local, servicio de grafo.

## Decisión

Actualmente memoria + export JSON. SQLite detrás de RepositoryPort después de validar UI.

## Consecuencias

JSON sencillo y auditable; no guardar raw por defecto. SQLite necesita transacciones, versiones/migraciones y borrado de evidencia referencial. No confundir formato export con base de datos.

## Revisión prevista

Al iniciar historia de sesiones/multi-run.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
