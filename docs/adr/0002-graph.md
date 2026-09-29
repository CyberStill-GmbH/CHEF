# ADR 0002: Grafo y procedencia

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

## Contexto

Hay duplicados y nombres compartidos; no debemos fusionar hosts por apariencia.

## Opciones

Grafo property local vs base de grafos externa vs listas sin relaciones.

## Decisión

Snapshot tipado en memoria; dirección/servicio con claves canónicas y SHA-256. Relationship observed; ExposurePath inferred. Observations inmutables en export.

## Consecuencias

Evidencia y cardinalidad explícitas; no Neo4j. Reimportación estable con reloj congelado; no deduplicar evidencias distintas como si fueran una. hostname/DNS serán entidades separadas.

## Revisión prevista

Al introducir endpoints o multi-run.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
