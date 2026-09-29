# API propuesta: posterior al núcleo offline

No hay servidor implementado. Contrato para historia W03; usar HTTP JSON sobre loopback, sin habilitar LAN por defecto. Token de sesión aleatorio, validación Origin y CORS restrictivo; sin cookies ambient ni endpoints que acepten URLs arbitrarias.

| Operación                                 | Entrada y salida                                                  | Fallos                                      |
| ----------------------------------------- | ----------------------------------------------------------------- | ------------------------------------------- |
| POST /v1/imports                          | multipart XML ≤ 2 MiB + policy validada; 202 {jobId,state:queued} | 400 input, 413 límite, 422 scope, 401 token |
| GET /v1/jobs/{id}                         | estado/counters; 200                                              | 404 ID desconocido                          |
| POST /v1/jobs/{id}/cancel                 | sin payload; 202 cancel-requested o 200 terminal                  | 404                                         |
| GET /v1/graphs/{id}/assets?cursor=&limit= | ≤100/por defecto; máximo500; {items,nextCursor,snapshotRevision}  | 400 cursor inválido, 409 revisión cambió    |
| GET /v1/graphs/{id}/relationships         | mismo mecanismo de paginación                                     | mismos fallos                               |
| GET /v1/graphs/{id}/export                | JSON Snapshot versionado                                          | 404, 409 no completo                        |
| DELETE /v1/graphs/{id}                    | confirma borrado de datos locales por ID; 204                     | 404                                         |

Error uniforme: `{code,message,requestId,retryable}` sin raw ni stack. OUT_OF_SCOPE nunca debe crear grafo parcial. Cursores opacos ligados a revisión estable; no offsets cambiantes sobre grafo vivo.

Estados: queued → running → completed / failed; queued/running → cancelling → cancelled. Terminales no vuelven a running; retry crea intento nuevo con vínculo al anterior. Eventual completed concurrente con cancel se resuelve con commit transaccional y estado único. Timeout/cancel conserva solo auditoría saneada, no relaciones a medias.

Pruebas de contrato pendientes: multipart válido/hostile; cancel mientras parser worker procesa; cursor al actualizar grafo; auth/Origin; export schema; 413 antes de leer todo; borrar grafo sin dejar evidencia huérfana. No publicar endpoint de scan hasta implementar el modelo de scope de red.
