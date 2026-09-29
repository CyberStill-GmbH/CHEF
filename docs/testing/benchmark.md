# Benchmark reproducible

```sh
npm run benchmark
```

Dataset original sintético: [lab.xml](../../fixtures/nmap/lab.xml); [etiquetas](../../fixtures/benchmark/labels.json). Tres observaciones open: dos duplicadas en host1 y una en host2. Hosts comparten hostname; esto es un control negativo de asociación por nombre. Puertos closed/open|filtered y host down no generan servicios. Etiquetas positivas/negativas referencian el orden del parser, documentado en el fixture.

Línea base: número de servicios observados sin correlación. CHEF produce servicios deduplicados y conserva todas las observaciones. Precision=TP/(TP+FP); recall=TP/positivos. Traceability cuenta entidades/aristas con referencias resolubles hasta Evidence. Dos reimportaciones con reloj fijo deben producir exportaciones idénticas. Warmup10 y muestras100; p50/p95 solo pipeline, sin IO, schema validation en el loop ni render.

La muestra es demasiado pequeña para acreditar utilidad real, rendimiento de UI o robustez estadística. Un resultado perfecto con un par positivo y dos negativos no justifica un claim de precisión universal. Los resultados reales del entorno se guardan en [verificación](../process/verification.md), separados de objetivos.

## Diseño ampliado, todavía no ejecutado

100 y 1 000 observaciones; fixtures versionados: repetición exacta, IPv6 equivalente, TCP vs UDP, hosts/virtual hosts que comparten IP, endpoints iguales con distinto método, nombres DNS repetidos, redirects y cambio temporal. Dos revisores etiquetan entidad/relación y resuelven desacuerdos; publicar criterios de labeling.

Controles negativos obligatorios: hostname compartido no fusiona direcciones; mismo puerto en hosts diferentes no fusiona; evidencia obsoleta no confirma alcance actual; open|filtered no equivale a open. Comparar tiempo de explicar relaciones frente a Nmap sin correlacionar y Site Map con 5 usuarios. Medir hallazgos útiles/revisados y relaciones irrelevantes; el motor actual produce exposiciones, no vulnerabilidades.

Registrar commit, Node/OS, CPU/RAM, tamaño input, entidades/aristas, número de repeticiones, mediana/p95, pico RAM, latencia hasta render y tasa de errores. No cambiar umbrales después de ver resultados sin registrar la razón.
