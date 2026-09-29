# Requisitos profesionales del repositorio

Se adoptan prácticas proporcionadas al riesgo y al equipo; no se afirma certificación ni cumplimiento completo de un estándar.

## Desarrollo seguro

[NIST SSDF SP800-218 v1.1](https://csrc.nist.gov/pubs/sp/800/218/final) orienta preparación del equipo, protección de código/artefactos, desarrollo y respuesta a vulnerabilidades. En CHEF: ownership/review, policy de ramas, permisos mínimos CI, lockfile, threat model, pruebas hostile y SECURITY. La revisión de dependencias y fuentes acompaña cada incremento.

[OWASP ASVS](https://owasp.org/projects/asvs) sirve para diseñar verificaciones de futura API/web, especialmente validación, acceso, datos y manejo de errores. Seleccionar requisitos aplicables cuando haya servidor; no declarar nivel ASVS por tener un documento. [CWE](https://cwe.mitre.org/) clasifica debilidades; la [matriz de CHEF](../security/cwe-controls.md) las enlaza a fronteras y pruebas, sin confundir clasificación con certificación. El core CLI necesita además verificación de IO/parser/scope que no queda cubierta por un checklist web genérico.

## Mantenibilidad

Modularidad por responsabilidades, SOLID demostrado en imports/pruebas, ADRs pequeños, contracts versionados, fuentes fechadas y requisitos vinculados a backlog. Evitar microservicios y clases/abstracciones sin una variación real. Modelo de errores tipado; dependencias exactas; setup reproducible; fixtures que un principiante pueda ejecutar.

## GitHub y CI

Ramas cortas/PR pequeños, issue templates con éxito/fallo, review cruzado del equipo. Checks Quality, Burp spike y [CodeQL](ci-security.md) antes de merge cuando estén configurados como requeridos; main protegida con checks, resolución de conversaciones y force-push/deletion bloqueados. El número de approvals debe ajustarse al acceso real del equipo; nunca inventar reviewers ni afirmar revisión independiente inexistente. [Commits y PRs](commits-and-prs.md) describen el proceso.

CI no usa pull_request_target ni secrets para ejecutar código del fork. Actions fijadas por SHA oficial, permisos contents:read y credenciales no persistidas. Renovar pines con Dependabot y revisar diffs. Auditar paquetes sin aplicar upgrades incompatibles a ciegas.

## Presentación y operación

Métricas falsables, dataset y control negativo, guion/recuperación offline, scope explícito y claims moderados. BApp Store añade condiciones propias; [checklist](../bapp/acceptance.md) tiene evidencia/estado, no simples casillas cumplidas por intención.
