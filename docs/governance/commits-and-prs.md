# Commits, ramas y pull requests de CHEF

Objetivo: cambios pequeños, auditables y fáciles de revertir, con evidencia del comportamiento. Esta guía aplica al equipo y colaboradores externos; [CONTRIBUTING](../../CONTRIBUTING.md) describe el recorrido completo.

## Ramas y commits

Crear rama desde `main` actualizado para cada issue/ADR; preferir `docs/<tema>` para documentación y `feature/<ID>-<tema>` o `fix/<ID>-<tema>` para código. `docs` es la rama de integración documental solicitada para esta preparación; tras el merge no debe usarse como desarrollo permanente. No hacer trabajo directo en `main` salvo merge revisado. Antes de editar: `git status`, leer [scope](../product/scope.md) y ADR relevante, preservar cambios ajenos.

Usar mensajes en imperativo y en inglés o español **de forma consistente dentro del PR**; forma recomendada `tipo(área): resultado concreto`. Tipos: `docs`, `feat`, `fix`, `test`, `refactor`, `ci`, `chore`. Ejemplos: `docs(adr): justificar PostgreSQL para proyectos multiusuario`; `test(core): fijar control negativo de hostname compartido`; `ci(codeql): analizar TypeScript y Java`. El asunto debe explicar el cambio, no “updates”; cuerpo opcional con motivo/compatibilidad. No incluir secretos, datos de clientes ni hashes de archivos sensibles en mensajes. Separar commits por intención revisable: decisión/contrato, implementación+pruebas, CI/docs; no fragmentar una corrección inseparable solo por estética. No reescribir historia compartida sin coordinación.

Un cambio de contrato público incluye schema, ejemplo, prueba y nota de compatibilidad en el mismo PR; un cambio de regla incluye fixture positivo/negativo y versión. Un ADR nuevo se abre con [la guía](../adr/README.md), no se edita una decisión anterior para ocultar su historia. La revisión del diff antes de commit verifica que solo hay archivos previstos.

## PR pequeño y review

Una PR debe resolver una historia/decisión y tener descripción que un revisor nuevo pueda entender: **problema/trigger, comportamiento antes/después, alcance real, contrato/ADR afectado, Given/When/Then positivo y negativo, validación ejecutada, riesgos y pendientes**. Incluir capturas solo para UI real y datos sintéticos. Indicar explícitamente si algo es `implementado`, `propuesto` o `validado manualmente`. No mezclar UI, parser, DB y Burp en una PR masiva salvo un corte vertical mínimo que exija integración; dividir por contrato y dependencias. El autor solicita review de alguien distinto: César↔Diego/Jhojan según área, con el [mapa de responsabilidades](../plan/team.md). La inicialización automatizada no sustituye esa revisión.

Gate antes de merge: `npm ci`, `npm run check`; para Java/Montoya, `mvn -B -f extensions/burp/pom.xml clean verify` y matriz manual separada cuando se cambie su comportamiento; CI `Quality`, `Burp spike` y `CodeQL` verdes según archivos. Revisar alertas CodeQL relevantes y documentar falsos positivos con explicación, no suprimir por conveniencia. El análisis estático no cubre autorización real, falsos enlaces ni UX. Probar scope, evidencia y acceso entre proyectos cuando corresponda. Nunca ejecutar reconocimiento activo solo para “validar CI”.

Responder comentarios con cambio o razón técnica; resolver conflictos sobre base actual; repetir pruebas afectadas. El merge debe conservar trazabilidad del PR/commits, con título que describa el resultado. No etiquetar release ni anunciar despliegue por haber fusionado documentos. Después del merge, verificar checks en `main` y que el README/links representen el estado real.

## Plantilla y política de seguridad

La [plantilla de PR](../../.github/PULL_REQUEST_TEMPLATE.md) es el mínimo, no un formulario mecánico. Reportar bugs con versión/commit, fixture sintético mínimo y esperado/observado; vulnerabilidades por [SECURITY](../../SECURITY.md). Para contribuciones de terceros, confirmar licencia MIT del código original y términos de cada fuente/API; una herramienta open source no concede derecho automático a almacenar/redistribuir sus datos.
