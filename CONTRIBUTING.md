# Contribuir

Consulta [desarrollo local](docs/development.md), [alcance](docs/product/scope.md) y [backlog](docs/plan/backlog.md).

Usa `codex/<tema>` o `feature/<ID>-<tema>` y PR hacia `main`. La rama `docs` existe por petición del propietario para esta preparación inicial; puede incluir los ejemplos ejecutables que sustentan las decisiones. Conserva su historial después del merge.

Cada PR explica problema, comportamiento resultante, validación y límites. Evita cambios cosméticos mezclados con reglas de correlación. Una persona implementa y otra revisa; comparte los contratos con Diego y acompaña los cambios de Jhojan. Cambios públicos de APIs requieren discusión en el ADR correspondiente.

Antes de pedir revisión: `npm ci`, `npm run check`, demo del fixture afectado, ausencia de datos reales y actualización del contrato si cambia. No uses el porcentaje de cobertura como sustituto de controles negativos. Una prueba que demuestra un bug debe fallar antes del arreglo.

Reporta bugs con versión/commit, plataforma, fixture mínimo anonimizado y resultado esperado/real. Reporta vulnerabilidades según [SECURITY](SECURITY.md). Al contribuir, confirma que puedes aportar el código bajo MIT; no copies código de Nmap ni ejemplos Montoya sin revisar sus términos.
