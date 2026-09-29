# Demo de cinco minutos

## Preparación

Máquina local con Node24, dependencies ya provisionadas; `npm ci` necesita red la primera vez. `npm run check` y `npm run build` previamente. Sin XML real, Internet, API keys ni Nmap instalado. Snapshot respaldo en `docs/contracts/examples/snapshot.json`.

## Guion objetivo para CSH

1. 00:00–00:35: problema de datos dispersos; mostrar archivo sintético y scope loopback.
2. 00:35–01:20: importar y mostrar estados/límites; núcleo actual usa CLI, futura W01 muestra grafo.
3. 01:20–02:20: seleccionar servicio duplicado; dos observaciones, una entidad; inspeccionar hash/locator.
4. 02:20–03:15: mostrar host2 con el mismo hostname y explicar por qué no se fusiona.
5. 03:15–04:10: exposición inferida y regla; no afirmar vulnerabilidad ni disponibilidad actual.
6. 04:10–05:00: export/reimport y benchmark; explicar límite del dataset y siguiente fase Burp autónoma.

El guion completo con UI es una meta pendiente, no ensayo realizado. La demo actual verificable produce grafo JSON:

```sh
npm run build
node dist/apps/cli/src/main.js fixtures/nmap/lab.xml fixtures/scope/lab.json
npm run benchmark
```

Control de fallo: importar `fixtures/nmap/hostile-xxe.xml` con mismo scope; esperar INVALID_INPUT y stdout vacío. Recuperación: abrir snapshot respaldo validado y explicar pipeline. Si falla la UI, no simularla con resultados inventados; mostrar la CLI y el JSON.

Tres ensayos cronometrados antes30-10: registrar duración, commit, tester, problemas y recuperación. César presenta razonamiento/core; Diego interacción/UI; Jhojan parsing/fixture/export. El responsable técnico no debe monopolizar la presentación.
