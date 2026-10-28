# ADR 0005: Comunicación con la web

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

Vigencia: la CLI y el snapshot TypeScript siguen siendo reales; la API loopback/SSE era exploratoria. La dirección actual es la aplicación de escritorio propuesta en [ADR 0012](0012-escritorio-tauri-rust.md). Tauri IPC, eventos y canales aún no están implementados; se medirán volumen, latencia, cancelación y límites antes de seleccionar el transporte de logs.

## Contexto

React debe recibir estado sin acoplarse a parser/IO.

## Opciones

Archivo importado en browser, API loopback+SSE, WebSocket.

## Decisión

Primera UI consume snapshot validado; API loopback y SSE previstos para ejecución por lotes.

## Consecuencias

Sin servidor ficticio. Token/Origin antes de API. Diffs revisados y bounded queue; no actualizar layout por cada observación.

## Revisión prevista

Al validar W01 y benchmark de 1 000 nodos.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
