# ADR 0004: Persistencia incremental

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

Vigencia: describe el incremento CLI offline implementado. La nueva visión web multiusuario con PostgreSQL/Prisma en Node.js y correlación Go se documenta en [ADR 0011 propuesto](0011-node-go-boundary.md); no afirmar que SQLite o PostgreSQL estén implementados. Al aceptar 0011, actualizar este estado histórico como supersedido para persistencia web.

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
