# ADR 0005: Comunicación con la web

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

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
