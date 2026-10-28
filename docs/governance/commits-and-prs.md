# Commits, ramas y pull requests de CHEF

Objetivo: cambios pequeños, auditables y fáciles de revertir, con evidencia del comportamiento. Esta guía aplica al equipo y colaboradores externos; [CONTRIBUTING](../../CONTRIBUTING.md) describe el recorrido completo.

## Ramas y commits

Crear una rama corta desde `main` actualizado para cada incremento revisable. CHEF usa `develop/<componente>` para desarrollo, `docs/<tema>` para documentación y `fix/<tema>` para correcciones delimitadas. No trabajar ni integrar directamente en `main`. Antes de editar: `git status`, leer [scope](../product/scope.md) y ADR relevante, preservar cambios ajenos. Preferir PR independientes con base `main`; apilar PR solo si existe una dependencia real. Integrada la dependencia, actualizar la siguiente PR contra `main`, repetir los checks y solicitar revisión sobre ese diff actualizado.

Usar mensajes en imperativo y en inglés o español **de forma consistente dentro del PR**; forma recomendada `tipo(área): resultado concreto`. Tipos: `docs`, `feat`, `fix`, `test`, `refactor`, `ci`, `chore`. Ejemplos: `docs(adr): justificar PostgreSQL para proyectos multiusuario`; `test(core): fijar control negativo de hostname compartido`; `ci(codeql): analizar TypeScript y Java`. El asunto describe el resultado, no “updates”. Cada commit debe ser una unidad coherente y revisable; separar intenciones sin fragmentar correcciones inseparables. No incluir secretos, datos de clientes ni hashes sensibles. No reescribir historia publicada después de pedir review; coordinar cualquier excepción.

Un cambio de contrato público incluye schema, ejemplo, prueba y nota de compatibilidad en el mismo PR; un cambio de regla incluye fixture positivo/negativo y versión. Un ADR nuevo se abre con [la guía](../adr/README.md), no se edita una decisión anterior para ocultar su historia. La revisión del diff antes de commit verifica que solo hay archivos previstos.

## PR pequeño y review

Una PR debe resolver una historia/decisión y tener descripción que un revisor nuevo pueda entender: **problema/trigger, comportamiento antes/después, alcance real, contrato/ADR afectado, Given/When/Then positivo y negativo, validación ejecutada, riesgos y pendientes**. El título usa `tipo(área): resultado concreto`. Incluir capturas solo para UI real y datos sintéticos. Indicar explícitamente si algo es `implementado`, `propuesto` o `validado manualmente`. No mezclar UI, parser, DB y Burp en una PR masiva salvo un corte vertical mínimo que exija integración; dividir por contrato y dependencias. El autor solicita al menos una aprobación de una persona distinta y vigente tras el último cambio, según el [mapa de responsabilidades](../plan/team.md). La inicialización automatizada no sustituye esa revisión.

Gate antes de merge: `npm ci`, `npm run check`; para Java/Montoya, `mvn -B -f extensions/burp/pom.xml clean verify` y matriz manual separada cuando se cambie su comportamiento; CI `Quality`, `Burp spike` y `CodeQL` verdes según archivos. Revisar alertas CodeQL relevantes y documentar falsos positivos con explicación, no suprimir por conveniencia. El análisis estático no cubre autorización real, falsos enlaces ni UX. Probar scope, evidencia y acceso entre proyectos cuando corresponda. Nunca ejecutar reconocimiento activo solo para “validar CI”.

Responder comentarios con cambio o razón técnica; resolver conflictos sobre la base actual; repetir los checks afectados. Mantener el PR en borrador mientras falte implementación, evidencia, aprobación o CI verde. Integrar solo tras aprobación independiente vigente y checks requeridos verdes, usando el método habilitado por el repositorio y un título de integración que conserve el resultado y la trazabilidad al PR. No integrar PR apiladas fuera de orden. No etiquetar release ni anunciar despliegue por haber fusionado documentos. Después del merge, verificar checks en `main` y que README/enlaces representen el estado real.

## Plantilla y política de seguridad

La [plantilla de PR](../../.github/PULL_REQUEST_TEMPLATE.md) es el mínimo, no un formulario mecánico. Reportar bugs con versión/commit, fixture sintético mínimo y esperado/observado; vulnerabilidades por [SECURITY](../../SECURITY.md). Para contribuciones de terceros, confirmar licencia MIT del código original y términos de cada fuente/API; una herramienta open source no concede derecho automático a almacenar/redistribuir sus datos.
