# Requisitos y trazabilidad

Actualizado 29-09-2026. `implementado` significa código y prueba del incremento actual; `propuesto` describe el producto web y **no** acredita entrega. La prioridad P0/P1/P2/P3 y responsable están en el [Product Backlog](../plan/backlog.md); el [Sprint Backlog](../proposals/2026-09-scrum-to-oct31.md) selecciona trabajo por capacidad. Los objetivos numéricos son hipótesis hasta medición.

| ID     | Comportamiento observable                                                                                     | Estado                                   | Backlog / verificación                 |
| ------ | ------------------------------------------------------------------------------------------------------------- | ---------------------------------------- | -------------------------------------- |
| FR-01  | Importar subset UTF-8 Nmap autorizado, solo hosts/puertos soportados y en scope, sin salida parcial al fallar | implementado                             | regresión WB06/WB07; fixture XML/CLI   |
| FR-02  | Resolver cada activo/relación a Observation → Evidence → digest/locator/run                                   | implementado                             | WB04/WB09; schema/golden               |
| FR-03  | IDs deterministas y deduplicación conservadora; hostname compartido no fusiona hosts                          | implementado                             | WB07/WB09; positivo/negativo           |
| FR-04  | Grafo y lista equivalentes, panel de regla/evidencia/fecha, estados accesibles                                | propuesto                                | WB10/WB11; teclado, etiqueta hostil    |
| FR-05  | Exportar snapshot versionado con referencias válidas; rechazar versión/desreferencia inválidas                | núcleo implementado; web propuesta       | WB12; golden/reimportación             |
| FR-06  | Distinguir observado, inferido, candidato, conflictivo; no afirmar vulnerabilidad sin prueba                  | núcleo parcial; multifuente propuesto    | WB09/WB11; labels y control negativo   |
| FR-07  | Enlazar Nmap activo importado y una fuente pasiva autorizada por regla versionada/temporal                    | propuesto, fuente bloqueada              | WB02/WB08/WB09; fixture multifuente    |
| FR-08  | Crear proyecto y limitar consultas/import/export a miembros autorizados                                       | propuesto                                | WB05/WB13; dos usuarios/proyectos      |
| FR-09  | Mostrar fuente, frescura, cuota/error y exclusiones de normalización sin inventar resultados                  | propuesto                                | WB07/WB08/WB11; errores/duplicados     |
| FR-10  | Descargar datos de evidencia versionados y, posterior, imagen de vista filtrada                               | propuesto                                | WB12; contrato/seguridad de SVG        |
| FR-11  | Spike Burp mantiene conformance Java/TS; BApp completa se evalúa después                                      | spike parcial                            | WB19; matriz manual separada           |
| NFR-01 | Sin red/escaneo activo por defecto; policy explícita nunca se amplía desde dato importado                     | implementado core; web por probar        | WB06/WB08; scope negativo              |
| NFR-02 | XML ≤2 MiB, ≤10 000 registros, scope ≤64 KiB y profundidad ≤32; API futura limita upload/jobs                 | core implementado; API propuesta         | WB06/WB07; hostile/boundary            |
| NFR-03 | Dominio independiente de IO, framework, Prisma, Montoya; application depende de puertos                       | implementado; ampliar                    | WB01/WB05; check:architecture + review |
| NFR-04 | Latencia/volumen de UI y actualización medidos antes de declarar tiempo real                                  | por medir                                | WB11/WB16; hardware/dataset            |
| NFR-05 | Importación atómica, idempotente, cancelación real solo al usar worker                                        | core parcial; web propuesta              | WB05/WB06; rollback/idempotencia       |
| NFR-06 | Aislamiento de proyectos, sesiones y export sin IDOR                                                          | propuesto                                | WB05/WB13; controles CWE-862/639       |
| NFR-07 | Lint, types, pruebas, schemas, docs, build y CodeQL en CI                                                     | checks existentes; CodeQL en integración | WB01–WB19; workflows/evidencia GitHub  |
| NFR-08 | Logs sin payload/secretos; retención, backup/restore y borrado definidos                                      | CLI parcial; web propuesta               | WB14; ensayo operacional               |
| NFR-09 | Demo reproducible con estado implementado/pendiente claro y comparación manual                                | propuesto                                | WB15; runbook y actas                  |
| NFR-10 | Montoya no bloquea UI ni exige servicio no documentado; unload/manual probado                                 | spike parcial                            | WB19; pruebas manuales pendientes      |

La [matriz CWE](../security/cwe-controls.md) conecta riesgos de XML, XSS, IDOR y SSRF a pruebas. Cambios de contrato público exigen schema, ejemplo, prueba y nota de compatibilidad. Un proveedor de OSINT que devuelve “sin datos” por cuota/error incumple FR-09. No se declara conformidad total con ASVS/SSDF/CodeQL por incluir estas filas.
