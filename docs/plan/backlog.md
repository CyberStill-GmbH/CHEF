# Backlog priorizado

Cuatro épicas y orden por dependencia. Tamaños en horas netas para integración/revisión por el equipo, no horas ya ejecutadas. **Total 94 h**. Estado inicial de equipo: Todo; los componentes core preparados automáticamente necesitan revisión/adopción antes de pasar a Done.

## C01: Importar subset Nmap normalizado

Épica: Ingestión/evidencia. Sprint S1. Tamaño 5 h. Dueño Jhojan; revisor César. Dependencias: —. Requisitos: FR-01,NFR-02 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero importar subset nmap normalizado para conservar un análisis explicable y revisable.

- Éxito: Given un fixture válido, When se importa con policy explícita, Then emite puertos open normalizados.
- Fallo: Given XML hostile/oversize, When se importa, Then error sin snapshot.
- Pruebas/entregable: parser/CLI hostile, límites, UTF-8.

## C02: Construir evidencia e identidad determinista

Épica: Grafo/correlación. Sprint S1. Tamaño 6 h. Dueño César; revisor Diego. Dependencias: C01. Requisitos: FR-02,FR-06,NFR-03 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero construir evidencia e identidad determinista para conservar un análisis explicable y revisable.

- Éxito: Given Observation válida, When se correlaciona, Then cada relación resuelve evidencia y regla.
- Fallo: Given relación sin evidencia o vulnerabilidad marcada, When se valida, Then se rechaza.
- Pruebas/entregable: referencias, schema, arquitectura.

## C03: Deduplicar sin falsas fusiones

Épica: Grafo/correlación. Sprint S1. Tamaño 2 h. Dueño Jhojan; revisor César. Dependencias: C01,C02. Requisitos: FR-03 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero deduplicar sin falsas fusiones para conservar un análisis explicable y revisable.

- Éxito: Given servicio repetido, When se fusiona, Then una entidad conserva dos observaciones.
- Fallo: Given hostname compartido con IP distinta, When se fusiona, Then servicios separados.
- Pruebas/entregable: fixture etiquetado, idempotencia.

## W01: Explorar snapshot en React y lista

Épica: Exploración/export. Sprint S1. Tamaño 6 h. Dueño Diego; revisor César. Dependencias: C02. Requisitos: FR-04 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero explorar snapshot en react y lista para conservar un análisis explicable y revisable.

- Éxito: Given JSON válido, When se abre, Then grafo/lista seleccionable por teclado.
- Fallo: Given JSON inválido/vacío, When se abre, Then error/empty sin render parcial.
- Pruebas/entregable: UI fixture, teclado, schema.

## Q01: Preparar CI y errores seguros

Épica: Calidad/demo. Sprint S1. Tamaño 2 h. Dueño César + Diego; revisor Jhojan. Dependencias: C01. Requisitos: NFR-01,NFR-07,NFR-08 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero preparar ci y errores seguros para conservar un análisis explicable y revisable.

- Éxito: Given PR válido, When CI corre, Then checks verdes y build disponible.
- Fallo: Given test/contrato roto, When CI corre, Then bloquea integración.
- Pruebas/entregable: workflow, lint, docs, stderr.

## W02: Mostrar evidencia e incertidumbre

Épica: Exploración/export. Sprint S2. Tamaño 7 h. Dueño Diego; revisor César. Dependencias: W01. Requisitos: FR-04,FR-06 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero mostrar evidencia e incertidumbre para conservar un análisis explicable y revisable.

- Éxito: Given entidad seleccionada, When se inspecciona, Then muestra observaciones/hash/rule.
- Fallo: Given label HTML o evidencia faltante, When se visualiza, Then texto plano/error explícito.
- Pruebas/entregable: UI adversarial, estados.

## W03: Implementar frontera de ejecución local

Épica: Exploración/export. Sprint S2. Tamaño 7 h. Dueño César; revisor Diego. Dependencias: W01,C01. Requisitos: FR-04,NFR-01 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero implementar frontera de ejecución local para conservar un análisis explicable y revisable.

- Éxito: Given sesión autenticada, When se sube fixture, Then job/snapshot según contrato.
- Fallo: Given Origin/token inválido o payload grande, When se llama API, Then rechazo sin trabajo.
- Pruebas/entregable: API contract, auth, bounded read.

## C04: Exportar e importar ejemplo versionado

Épica: Exploración/export. Sprint S2. Tamaño 4 h. Dueño Jhojan; revisor Diego. Dependencias: C02. Requisitos: FR-05 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero exportar e importar ejemplo versionado para conservar un análisis explicable y revisable.

- Éxito: Given snapshot válido, When se exporta, Then reader conserva IDs/evidencia.
- Fallo: Given versión desconocida/referencias inválidas, When se abre, Then rechazo legible.
- Pruebas/entregable: schema/golden/semantic reader.

## I02: Spike HTTP normalizado sujeto a gate

Épica: Ingestión/evidencia. Sprint S2. Tamaño 3 h. Dueño Jhojan; revisor César. Dependencias: C04. Requisitos: FR-07 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero spike http normalizado sujeto a gate para conservar un análisis explicable y revisable.

- Éxito: Given metadata sintética saneada, When se normaliza, Then endpoint tipado y sin query/auth.
- Fallo: Given input malformado o fuera de scope, When se procesa, Then error.
- Pruebas/entregable: fixtures HTTP y contrato1.1; si gate falla, mejorar fixture Nmap.

## W04: Hacer explorador accesible y acotado

Épica: Exploración/export. Sprint S3. Tamaño 7 h. Dueño Diego; revisor Jhojan. Dependencias: W02. Requisitos: FR-04,NFR-04 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero hacer explorador accesible y acotado para conservar un análisis explicable y revisable.

- Éxito: Given grafo grande, When se filtra, Then selección usable/lista equivalente.
- Fallo: Given buffer/render saturado, When se alcanza límite, Then resync/error visible.
- Pruebas/entregable: UI rendimiento, teclado/lectura.

## I03: Worker y cancelación en curso

Épica: Ingestión/evidencia. Sprint S3. Tamaño 7 h. Dueño César; revisor Diego. Dependencias: W03. Requisitos: NFR-05 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero worker y cancelación en curso para conservar un análisis explicable y revisable.

- Éxito: Given job activo, When se cancela, Then terminal <1s sin updates nuevos.
- Fallo: Given excepción/timeout, When termina worker, Then error saneado sin grafo parcial.
- Pruebas/entregable: integration worker/cancel.

## Q02: Benchmark ampliado y controles negativos

Épica: Calidad/demo. Sprint S3. Tamaño 7 h. Dueño Jhojan; revisor César. Dependencias: C03,W01. Requisitos: FR-03,NFR-04 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero benchmark ampliado y controles negativos para conservar un análisis explicable y revisable.

- Éxito: Given labels revisados, When se mide, Then TP/FP/recall/latencia con entorno.
- Fallo: Given control negativo fusionado, When se evalúa, Then falla gate y se registra.
- Pruebas/entregable: 100/1000 obs, resultados reales.

## Q03: Ensayar demo cinco minutos

Épica: Calidad/demo. Sprint S4. Tamaño 9 h. Dueño Los tres; revisor Revisión cruzada. Dependencias: W02,C04. Requisitos: NFR-06,NFR-09 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero ensayar demo cinco minutos para conservar un análisis explicable y revisable.

- Éxito: Given equipo desconectado, When se sigue runbook, Then3 ensayos completos.
- Fallo: Given archivo corrupto o UI rota, When se recupera, Then usa respaldo validado.
- Pruebas/entregable: ensayos cronometrados.

## Q04: Endurecer integración y release demo

Épica: Calidad/demo. Sprint S4. Tamaño 12 h. Dueño Los tres; revisor Revisión cruzada. Dependencias: Q02,I03. Requisitos: NFR-02,NFR-07,NFR-08 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero endurecer integración y release demo para conservar un análisis explicable y revisable.

- Éxito: Given build limpio, When se valida, Then sin defectos de evidencia/scope.
- Fallo: Given secret/raw/limit bug, When se revisa, Then no release hasta resolver.
- Pruebas/entregable: adversarial, CI, licenses.

## B01: Validar spike Montoya manualmente

Épica: Calidad/demo. Sprint S4. Tamaño 6 h. Dueño César + Jhojan; revisor Diego. Dependencias: I02. Requisitos: FR-08,NFR-10 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero validar spike montoya manualmente para conservar un análisis explicable y revisable.

- Éxito: Given Burp local, When se carga/selecciona, Then Observation/export útil.
- Fallo: Given out-of-scope/unload, When se intenta procesar, Then rechazo/worker termina.
- Pruebas/entregable: MB01–MB09, conformidad Java/TS.

## Q05: Preparar narrativa/capturas y paquete público

Épica: Exploración/export. Sprint S4. Tamaño 4 h. Dueño Diego; revisor Jhojan. Dependencias: Q03. Requisitos: NFR-09 ([trazabilidad](../product/requirements.md)).

Historia: como analista/colaborador quiero preparar narrativa/capturas y paquete público para conservar un análisis explicable y revisable.

- Éxito: Given demo validada, When se presenta, Then distingue hechos/hipótesis/futuro.
- Fallo: Given captura sensible o función pendiente, When se prepara material, Then se anonimiza/etiqueta.
- Pruebas/entregable: runbook, revisión de capturas.
