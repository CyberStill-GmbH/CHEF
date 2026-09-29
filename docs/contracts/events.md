# Eventos históricos propuestos

**Diferidos:** este diseño SSE de la antigua API loopback no está implementado ni forma parte de la primera vertical web. La nueva [API propuesta](web-api-proposal.md) comienza por importación/lectura con fecha visible; eventos se decidirán con una medición de latencia, reconexión y demanda real. Conservar esta exploración para un ADR futuro, sin asumir que un cliente puede depender de ella.

No implementados. SSE local para UI; no se necesita WebSocket bidireccional para el MVP. Envelope: `{schemaVersion,eventId,jobId,sequence,occurredAt,type,payload}`. Sequence aumenta por job; eventId estable para replay.

Tipos: run.started, observation.batch, graph.diff, run.progress, run.completed, run.failed, run.cancelled. Graph.diff contiene revision/baseRevision y arrays de upserts/removals; el frontend descarta duplicados, detecta gap y pide snapshot. En completed se anuncia el SHA-256 del snapshot canónico.

Batch inicial propuesto ≤100 observaciones o 250 ms; cola ≤1 000 items. Al superar el buffer no bloquear Burp ni perder eventos silenciosamente: pausar productor o devolver resync-required. No incluir cuerpos HTTP, credenciales ni rutas de disco. Cancelación entra por endpoint y desconecta workers; el evento solo refleja el estado ya confirmado.

Tests previstos: orden/deduplicación, reconexión Last-Event-ID, buffer agotado, job desconocido, snapshotRevision incompatible y cancelación sin updates posteriores al terminal. La UI renderiza labels como texto y no ejecuta enlaces importados.
