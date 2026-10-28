# Registro de decisiones arquitectónicas

Los ADRs preservan **por qué** se eligió una dirección, sus alternativas, consecuencias y evidencia de revisión. El código/contrato actual muestran **qué** se implementó; una dirección aceptada no equivale a una implementación. El [ADR 0012](0012-escritorio-tauri-rust.md) registra la dirección de producto aceptada por el propietario, con revisión técnica y toolchain pendientes. El contexto general está en [C4 del código actual](../architecture/overview.md), [arquitectura de escritorio](../architecture/desktop-tauri.md) y [seguridad CWE](../security/cwe-controls.md).

## Índice y vigencia

- [0001 · núcleo TypeScript](0001-core-language.md), aceptado para el incremento inicial; se conserva como base de compatibilidad mientras se propone el portado Rust.
- [0002 · identidad y grafo](0002-graph.md), aceptado para el incremento actual.
- [0003 · ingestión Nmap](0003-ingestion.md), aceptado para el incremento actual.
- [0004 · memoria y snapshot](0004-persistence.md), aceptado **solo** para el incremento TypeScript offline; la persistencia local de escritorio requiere una nueva decisión.
- [0005 · comunicación exploratoria](0005-web-events.md), antecedente de API web; la integración de escritorio se rige por la propuesta 0012.
- [0006 · Burp/Montoya](0006-burp-boundary.md), spike separado; BApp posterior.
- [0007 · explorador de grafo](0007-graph-ui.md), candidata para React, pendiente benchmark real.
- [0008 · plataforma web íntegramente TypeScript](0008-web-platform.md), **rechazado como propuesta futura** tras decisión del propietario; se conserva el análisis.
- [0009 · correlación multifuente](0009-evidence-correlation.md), **propuesto**; conserva los invariantes de 0002.
- [0010 · fuentes y ejecución activa](0010-source-integration.md), **propuesto**; la fase actual solo importa resultados autorizados.
- [0011 · frontera Node.js/Prisma/OAuth y motor Go](0011-node-go-boundary.md), **retirado por decisión del propietario el 18-10-2026**; se conserva como antecedente sin implementación.
- [0012 · escritorio Tauri, núcleo Rust y SurrealDB local](0012-escritorio-tauri-rust.md), **dirección aceptada por el propietario; validación técnica de integración pendiente**; fija gates de migración, seguridad y conformidad.

## Cuándo abrir un ADR

Abrirlo cuando una decisión cambie una frontera, lenguaje, fuente de verdad, contrato público, modelo de identidad, persistencia, seguridad multiusuario, modo de red o coste operativo difícil de revertir. Un ajuste local sin consecuencias arquitectónicas va en PR/issue, no en ADR. Antes de redactar: leer ADRs relacionados, código, contrato y pruebas; verificar fuentes primarias fechadas en [sources.json](../research/sources.json). Una decisión vieja no se borra ni reescribe para ocultar su contexto; un ADR nuevo la **supersede** explícitamente y el índice se actualiza.

## Cómo proponer y aceptar

1. Copiar [TEMPLATE.md](TEMPLATE.md) a `NNNN-slug.md` con el siguiente número, título preciso, fecha y estado **propuesto**. No reutilizar números ni llamarlo aceptado por incluirlo en main.
2. Escribir problema, restricciones/objetivos medibles, estado implementado, opciones realistas (incluida conservar actual), decisión recomendada, por qué, consecuencias/costes y caminos de reversión. Explicar quién es dueño de la decisión y qué evidencia falta.
3. Enlazar contrato/esquema/fixture afectado, modelo de amenazas/CWE cuando proceda, alternativa descartada y prueba Given/When/Then. Si cambia wire, incluir schema, ejemplo, prueba y nota de compatibilidad en el PR correspondiente.
4. Pedir revisión de otra persona del equipo y de los responsables afectados. Registrar objeciones/resolución en PR; el Product Owner cierra prioridad, el responsable técnico valida viabilidad y el reviewer verifica evidencia. Ejecutar `npm run check`.
5. Cambiar a **aceptado** solo tras consenso documentado y plan de migración. Estados permitidos: `propuesto`, `aceptado`, `rechazado`, `supersedido por ADR NNNN`. Una aceptación no afirma que esté implementado; registrar aparte el estado de código/tests/despliegue.

Revisar un ADR cuando cambian supuestos, una métrica invalida la elección o se propone un reemplazo. Las revisiones editoriales menores conservan historial Git; cambios sustanciales generan ADR nuevo. En cada PR arquitectónico, enlazar el ADR y actualizar diagramas C4/amenazas que cambiaron. Una rama de propuesta puede contener varios ADRs de investigación, pero la implementación se divide en PRs pequeños con criterio Given/When/Then.
