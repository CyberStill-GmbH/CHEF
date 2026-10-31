# Transporte local Node–Go

Implementado y probado localmente en la rama de integración; pendiente de revisión humana del [ADR 0011](../adr/0011-node-go-boundary.md), CI remoto y despliegue. No cambia la aceptación del ADR ni incorpora nuevas reglas multifuente.

## Recorrido

`POST /projects/:id/imports` autentica al usuario. `ImportIntoProject` comprueba membresía y rol antes de parsear. `ImportEvidence.prepare` valida XML y scope y construye observaciones canónicas de esa importación. El puerto `CorrelationEngine` entrega esos datos al adaptador `GoCorrelationEngine`; la aplicación no importa procesos, filesystem ni Prisma.

El adaptador valida `CorrelationJob v0` con AJV y comprueba scope y referencias a evidencia antes de invocar un ejecutable de configuración confiable con `spawn`, sin shell. Solo transmite el JSON por stdin. El proceso no hereda credenciales de GitHub, sesión ni base de datos; en Windows recibe únicamente las variables de sistema necesarias. No recibe argumentos derivados del XML ni del cliente HTTP.

Tras cerrar el proceso con código cero, Node valida el esquema de salida, versión y reloj, IDs estables, referencias, estados compatibles y métricas. Convierte `status` en `assertion` para conservar `Snapshot 1.0.0`. La publicación utiliza el `SnapshotStore` existente y solo ocurre después de esa validación. La CLI conserva la correlación TS como referencia; la API compuesta en `main.ts` usa Go y no recurre a TS si Go falla.

## Límites y fallos

| Control                                            | Valor predeterminado / comportamiento                              |
| -------------------------------------------------- | ------------------------------------------------------------------ |
| Entrada del motor                                  | 8 MiB de JSON UTF-8; rechazo antes de crear el proceso             |
| stdout                                             | 8 MiB; exceso termina el proceso y rechaza el resultado            |
| stderr                                             | 8 MiB; se cuenta y descarta, nunca se reenvía al cliente ni al log |
| Tiempo                                             | 10 segundos por proceso; terminación forzada                       |
| Concurrencia                                       | 2 procesos por instancia del adaptador; sin cola ilimitada         |
| Saturación                                         | `UPSTREAM_UNAVAILABLE` / HTTP 503                                  |
| Crash, timeout, salida inválida, referencias rotas | `UPSTREAM_ERROR` / HTTP 502, sin publicación                       |
| Cancelación                                        | `CANCELLED`; desconexión del cliente aborta la correlación         |

La plaza de concurrencia se libera al cerrar el proceso y sus streams. Una cancelación anterior al guardado no publica el snapshot. Una transacción de persistencia ya iniciada sigue las garantías del adaptador de almacenamiento existente: este cambio no añade rollback distribuido por desconexión. Los límites pueden fijarse al construir el adaptador, nunca desde la petición. El límite XML de la API y el de scope siguen aplicándose por separado.

## Ejecutar y comprobar

Se requiere Node 24 y Go compatible con `engine/go.mod`:

```sh
npm ci
npm run engine:build
npm run check
npm run engine:test
cd engine
go vet ./...
go test ./...
go test -run='^$' -fuzz=FuzzValidateJob -fuzztime=20s ./internal/correlation
go test -run='^$' -fuzz=FuzzCorrelate -fuzztime=20s ./internal/correlation
```

`npm run engine:build` genera `artifacts/bin/correlate` (`correlate.exe` en Windows). La API usa esa ruta por defecto; `CHEF_GO_BINARY` permite elegir un binario confiable. Es configuración del operador y no una ruta enviada por el cliente. Para arrancar la API se mantienen sus variables OAuth/sesión existentes. El Dockerfile compila Go con `CGO_ENABLED=0` y copia el ejecutable a la imagen Node, que corre como usuario `node`.

Para probar la imagen: `docker build -t chef-go-integration:local .` y `node scripts/smoke-go-container.mjs`. La prueba inicia un contenedor descartable sin red externa, levanta la composición real de la API e importa el fixture por HTTP con el binario empaquetado. Usa sesión sintética y almacenamiento en memoria: no verifica OAuth real, PostgreSQL ni Railway.

La suite `tests/go-correlation.test.ts` prueba conformidad contra TS con el binario real, importación HTTP y aislamiento, límites, crash, timeout, cancelación, diagnóstico saneado y conservación del snapshot anterior ante fallos. Los procesos falsos se usan solo para provocar fallos; la prueba positiva invoca Go real. Quality compila el motor antes de ejecutar la suite. CodeQL incluye Go mediante build manual.

Para medir el transporte local: `npm run engine:benchmark`. El resultado declara corpus, CPU, plataforma, latencias y RSS observado del proceso Node; no confundir ese RSS con la memoria máxima del proceso Go ni extrapolar el fixture a producción.

## Lo que sigue pendiente

Revisión técnica del contrato/seguridad/operación, ejecución de CodeQL en GitHub y prueba en el entorno de despliegue. El contrato v0 reserva estados `candidate` y `conflicted`, pero esta integración solo acepta `observed` e `inferred` de `open-service/v1` para mantener el snapshot actual. Las reglas multifuente y una nueva versión de snapshot requieren su propio incremento. La conformidad por fixtures no prueba equivalencia exhaustiva del validador estructural Go con JSON Schema.
