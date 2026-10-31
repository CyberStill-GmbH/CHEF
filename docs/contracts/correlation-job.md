# Contrato CorrelationJob v0 (frontera Node ↔ Go)

Estado: **propuesto, v0, no aceptado todavía**. Implementa el primer paso que exige el [ADR 0011](../adr/0011-node-go-boundary.md): definir el contrato (JSON Schema, ejemplos positivo/negativo, validadores y fixtures de conformidad) **antes** de escribir el motor Go. No reinterpreta `Snapshot 1.0.0`: un snapshot multifuente requeriría otra versión.

## Qué define

- **Entrada** — [`correlation-job.schema.json`](correlation-job.schema.json): lo que la API Node envía al motor de correlación. Observaciones ya canónicas y validadas, `projectScope` explícito, `asOf` (reloj fijo) y `ruleSetVersion`. El motor **no** recibe credenciales, **no** consulta proveedores y **no** accede a la base (ADR 0011).
- **Salida** — [`correlation-result.schema.json`](correlation-result.schema.json): assets, relationships, paths y `metrics` de deduplicación. Determinista: el mismo trabajo en cualquier orden de entrada produce el mismo resultado.

Ejemplos: [`examples/correlation-job.json`](examples/correlation-job.json) · [`examples/correlation-result.json`](examples/correlation-result.json) (positivos, derivados de [`examples/snapshot.json`](examples/snapshot.json) para que sirvan de conformidad) y [`examples/correlation-job.invalid.json`](examples/correlation-job.invalid.json) · [`examples/correlation-result.invalid.json`](examples/correlation-result.invalid.json) (negativos). `npm run check:docs` valida que los positivos pasan y los negativos fallan.

## Invariantes (del ADR 0011)

- **Una regla de superficie nunca confirma una vulnerabilidad.** El esquema fija `isConfirmedVulnerability` a `const: false`: cualquier `path` que afirme `true` es inválido por contrato, no solo por convención. Es la traducción a esquema del criterio 3 del ADR.
- **`status` ∈ `observed | inferred | candidate | conflicted`.** La regla `open-service/v1` solo emite `observed` (relaciones) e `inferred` (paths). `candidate`/`conflicted` quedan reservados para reglas futuras (p. ej. hostname/IP compartido o CT histórico), que deben mostrar pruebas en vez de confirmar propiedad o alcance.
- **IDs deterministas y estables.** Los IDs (`asset:`, `relationship:`, `path:`, `observation:`, `evidence:`, `run:`) son `sha256` y no cambian sin contrato versionado y prueba de compatibilidad.

## Correspondencia con el modelo TypeScript

La implementación TS aceptada (`packages/domain/src/model.ts` y `correlate.ts`) es la **referencia de conformidad**. El mapeo para v0:

| Modelo TS                                | CorrelationResult v0                              | Nota                                                                                               |
| ---------------------------------------- | ------------------------------------------------- | -------------------------------------------------------------------------------------------------- |
| `Relationship.assertion: "observed"`     | `relationship.status: "observed"`                 | mismo valor, nombre alineado al ADR                                                                |
| `ExposurePath.assertion: "inferred"`     | `path.status: "inferred"`                         | ídem                                                                                               |
| `ExposurePath.ruleId: "open-service/v1"` | `ruleId: "open-service/v1"` + `ruleVersion: "v1"` | el ADR pide `ruleId/version` explícitos                                                            |
| (no existe en el snapshot)               | `metrics.duplicateObservations`                   | observaciones cuya clave de servicio `address\|protocol\|port` ya se había visto en orden canónico |

En el ejemplo, las dos observaciones de `127.0.0.1\|tcp\|8080` producen un único asset de servicio con dos `observationIds`; la segunda cuenta como una observación duplicada (`duplicateObservations: 1`).

## Compatibilidad

- `correlation-job/v0` y `correlation-result/v0` se versionan en el propio `schemaVersion`. Un cambio incompatible exige `v1` y nota de compatibilidad, sin reutilizar IDs existentes.
- Este contrato no fija el transporte (ejecutable local por `stdin/stdout`, servicio aparte, etc.): eso se valida en el spike del ADR 0011. Solo describe el _payload_.
- Implementado en `engine/`: validadores Go, regla `open-service/v1`, ejecutable local y pruebas contra los fixtures compartidos. La conformidad del fixture no demuestra equivalencia completa entre el validador Go y JSON Schema.
- Pendiente antes de aceptarlo: revisión humana del contrato, seguridad y operación conforme al ADR 0011. El spike de transporte y la integración con la API siguen pendientes.
