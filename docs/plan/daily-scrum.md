# Pauta diaria de trabajo y Daily Scrum

Propuesta fechada 29-09-2026. El [Product Backlog](backlog.md) contiene opciones ordenadas; el [Sprint Backlog](../proposals/2026-09-scrum-to-oct31.md) limita el trabajo elegido. Esta pauta convierte el objetivo en **siguientes entregables por persona**, no afirma que ya se hayan completado ni obliga a seguir una fecha cuando cambia una dependencia. César (API Node/motor Go y Product Owner), Diego (frontend/UX) y Jhojan (ingestión/fixtures/investigación) tienen 10 h/semana cada uno, incluidas reuniones/reviews. El nombre vigente en [equipo](team.md) es Jhojan.

## Cómo funciona el Daily

Encuentro de hasta 15 min cada día laborable, a hora acordada por el equipo. Cada integrante actualiza su tarjeta con: **resultado verificable desde ayer**, **próximo corte que cabe en ~2 h**, **bloqueo/decisión que necesita** y **PR/review pendiente**. No es un reporte al Product Owner; sirve para inspeccionar avance al objetivo del sprint y adaptar el plan del día. Si una tarea queda bloqueada por fuente/contrato, no se sigue fabricando UI o datos: se trabaja en fixture, test o revisión independiente y se registra el bloqueo. No contar presencia en Daily como prueba de trabajo terminado.

El tablero `Ready → In progress → Review → Done` tiene una tarjeta en progreso por persona. César ordena el Product Backlog, el equipo negocia el Sprint Backlog y el reviewer cierra calidad. Cada tarjeta enlaza ID WB, Given/When/Then, fuente, estimación, evidencia de prueba y PR. Los estimados de abajo son **orientación de secuencia**; se dividen/reordenan al planning tras el gate de fuente y capacidad.

## Sprint 1 · decisión y contrato (29-09 a 05-10)

| Día de trabajo | César · backend/correlación                                       | Diego · frontend/UX                                                           | Jhojan · ingestión/investigación                                       |
| -------------- | ----------------------------------------------------------------- | ----------------------------------------------------------------------------- | ---------------------------------------------------------------------- |
| 1              | Revisar ADR 0011, puertos y coste Node–Go; abrir WB01             | Revisar tareas del analista, rutas de pantalla y accesibilidad                | Revisar selección Common Crawl/RDAP/RIPEstat y límites                 |
| 2              | Spike transporte Go + API Node/Prisma, riesgo OAuth/IDOR          | Wireframe proyecto→import→evidencia, con lista equivalente                    | Verificar términos/acceso/cuota Common Crawl y dominio autorizado      |
| 3              | Contrato mínimo de proyecto/observación/job Go para fixture       | Probar dos variantes de panel con fixture Nmap, anotar errores de comprensión | Fixture multifuente sintético rotulado y labels de enlace/falso enlace |
| 4              | Revisar contrato con Diego/Jhojan; dividir WB04                   | Fijar DTO/estados vacío/error con César; review de ADR                        | Prueba Nmap golden/hostname compartido; review de contrato             |
| 5              | Gate WB02/WB03: fuente, capacidad, corte web; publicar decisiones | Demo de prototipo navegable o mock con pendientes visibles                    | Publicar matriz de fuente, permiso, fixture y limitaciones             |

Salida: decisión y fixture trazable. Si la fuente no está autorizada, WB08 permanece bloqueada y el equipo reduce/renegocia la demo; no se dice “OSINT integrada”.

## Sprint 2 · importación persistida (06-10 a 12-10)

| Día | César                                                       | Diego                                                                    | Jhojan                                                    |
| --- | ----------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------- |
| 1   | Puerto de repositorio y modelo proyecto/run; PR pequeño     | Shell React y estados de importación basados en contrato                 | Casos Nmap de duplicado, estado ambiguo y rechazo atómico |
| 2   | Prisma/migración de dos proyectos de prueba                 | Vista servicio y cadena Observation→Evidence                             | Recuento/motivo de exclusión; golden y review Prisma      |
| 3   | Endpoint de importación que llama core existente            | Conectar carga de fixture a API o stub contractual, sin lógica de fusión | Test CLI intacta tras API y input hostile                 |
| 4   | Autorización básica por proyecto y transacción fallida      | Error/empty/loading, teclado y texto importado seguro                    | Prueba idempotencia/reimportación y control negativo      |
| 5   | Review cruzado, `npm run check`, demo local de XML→DB→vista | Recorrido de usuario y lista equivalente; reportar deuda                 | Informe de diferencias contra snapshot/IDs del core       |

Salida: Nmap desde UI local persiste/conserva evidencia solo si API/DB realmente funcionan. Si no, mostrar slice validado y registrar dónde se corta.

## Sprint 3 · fuente pasiva y enlace explicable (13-10 a 19-10)

| Día | César                                                  | Diego                                               | Jhojan                                                        |
| --- | ------------------------------------------------------ | --------------------------------------------------- | ------------------------------------------------------------- |
| 1   | Regla Go tipada fuerte/candidata, fixtures TS/Go       | Diseño visual observado/candidato/conflicto         | Adaptador Common Crawl aprobado o fixture sintético declarado |
| 2   | Bloqueo por proyecto/scope/tiempo, versión de regla Go | Panel con regla, dos evidencias y fechas            | Error/cuota/truncación, locator y fecha del proveedor         |
| 3   | Test Go positivo Nmap↔pasivo y falso enlace            | Grafo + lista sin recalcular identidad en React     | Datos de contradicción/historial, control negativo            |
| 4   | Review con Jhojan, crash/timeout y salida inválida     | Prueba teclado/lector y etiquetas hostiles          | Review de contrato/UI desde evidencia, `npm run check`        |
| 5   | Demo de relación y export preliminar                   | Validación con tarea real de analista si hay piloto | Informe de cobertura/frescura de fuente y limitaciones        |

Salida: primera relación defendible con fuente/fecha/regla; si solo es sintética, se comunica como validación de algoritmo/UI, no de OSINT real.

## Sprint 4 · operación y prueba de valor (20-10 a 30-10)

Se dispone de dos semanas laborales parciales; usar la primera para cerrar funcionalidad y la segunda para operación/reviews/ensayos. **Ninguna columna implica que OAuth/despliegue caben automáticamente**: el gate de capacidad decide si estas tarjetas entran o pasan al siguiente ciclo.

| Corte diario | César                                              | Diego                                                | Jhojan                                               |
| ------------ | -------------------------------------------------- | ---------------------------------------------------- | ---------------------------------------------------- |
| 1            | Scope de OAuth y membresías, callback y secretos   | Exportación JSON desde UI                            | Golden/export y snapshot externo                     |
| 2            | Test dos usuarios/proyectos e IDOR                 | Exportación de vista SVG/PNG solo si sobra capacidad | Tests de referencia rota y etiqueta hostil           |
| 3            | Configuración Railway/DB/migración, si aprobada    | Configuración Vercel/URL de equipo, si aprobada      | Ensayo de migración/fallo DB y reporte               |
| 4            | Backup/restore, logs seguros y rollback            | Accesibilidad final y responsive                     | Dataset/labels del ensayo comparativo                |
| 5            | Integración, revisión de CWE/ASVS aplicable        | Rúbrica y prueba piloto CHEF/manual                  | Ejecutar casos hostiles, negativos y `npm run check` |
| 6            | Corregir defectos que afectan evidencia/permisos   | Corregir comprensión/errores críticos                | Revisar cadenas Evidence→Observation en export       |
| 7            | Ensayo de demo en entorno realmente disponible     | Medir tiempo/pasos, registrar muestra real           | Registrar proveedor/permiso/versiones/fecha          |
| 8            | Freeze de versión y runbook recuperación           | Capturas sin datos de clientes, guion honesto        | Segundo ensayo, fallback y cierre de fixtures        |
| 9            | Gate final: CI, migración, seguridad y estado real | Recorrido final y lista de pendientes                | Evidencia de tests y limitaciones sin maquillar      |

El 31-10 se presenta el incremento validado; la meta web privada solo se anuncia cumplida con OAuth, aislamiento, backup y URL de equipo realmente probados. Los demás elementos se muestran como próximos pasos. La comparación manual registra tiempo, pasos y falsos enlaces; no inventa evaluadores ni resultados.

## Pauta para cambios

Cuando un bloqueo supera un día laborable, el owner lo marca en tablero, explica causa e impacto y propone alternativa; el Product Owner decide prioridad con el equipo, no reasigna silenciosamente el objetivo. El review de otra persona cuenta dentro de 10 h/semana. Antes de merge, cada PR sigue [CONTRIBUTING](../../CONTRIBUTING.md), mantiene core/CLI estable, pasa `npm run check` y enlaza escenario negativo. La [guía Scrum](https://scrumguides.org/scrum-guide.html) define Daily para inspeccionar/adaptar el Sprint Backlog, no para convertirse en un cronograma inflexible.
