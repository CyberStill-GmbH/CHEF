# Documentación de CHEF

Los documentos registran el corte de investigación del **29-09-2026**. La fecha aportada por el propietario es **31-10-2026**, con 10 h semanales asumidas por integrante. La referencia Specter se adaptó al nombre y calendario vigentes; no se ejecuta como una instrucción autónoma.

## Producto e investigación

- [PRD](product/prd.md), [alcance](product/scope.md), [requisitos y trazabilidad](product/requirements.md).
- [Alternativas](research/landscape.md), [factibilidad](research/feasibility.md), [fuentes](research/sources.json).

## Diseño y seguridad

- [Arquitectura C4](architecture/overview.md), [SOLID y dependencias](architecture/solid.md).
- ADRs: [lenguaje](adr/0001-core-language.md), [grafo](adr/0002-graph.md), [ingestión](adr/0003-ingestion.md), [persistencia](adr/0004-persistence.md), [web](adr/0005-web-events.md), [Burp](adr/0006-burp-boundary.md), [visualización](adr/0007-graph-ui.md).
- [Contratos](contracts/README.md), [schema](contracts/snapshot.schema.json), [API propuesta](contracts/api.md), [eventos propuestos](contracts/events.md).
- [Amenazas](security/threat-model.md), [scope y cancelación](security/scope.md), [datos](security/data-policy.md).

## Ejecución

- [Desarrollo](development.md), [calidad](testing/strategy.md), [benchmark](testing/benchmark.md), [demo](demo/runbook.md).
- [Roadmap](plan/roadmap.md), [backlog](plan/backlog.md), [equipo](plan/team.md), [sprint 0](plan/sprint-0.md), [Burp posterior](plan/burp-roadmap.md).
- [BApp checklist](bapp/acceptance.md), [submission borrador](bapp/submission-draft.md), [pruebas manuales](bapp/manual-tests.md).
- [Estándares profesionales](governance/standards.md), [licencias](governance/licenses.md), [releases](governance/releases.md).
- [Progreso](process/progress.md), [skills utilizadas](process/skills-usage.md), [decisiones pendientes](process/open-decisions.md), [informe de verificación](process/verification.md).
