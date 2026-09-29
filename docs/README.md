# Documentación de CHEF

La [propuesta de CHEF web antes de Burp](proposals/2026-09-scope-decision.md), su [matriz de cambios a ADRs y contratos](proposals/2026-09-adr-contract-deltas.md), [plan Scrum reestimado](proposals/2026-09-scrum-to-oct31.md), [issues sugeridas](proposals/2026-09-issues.md), [investigación de reconocimiento y correlación](research/recon-correlation-2026-09.md) y [brechas BApp](proposals/2026-09-bapp-gaps.md) están en una rama de revisión. No sustituyen los ADRs y contratos aceptados hasta que el equipo adopte la propuesta.

Los documentos registran el corte de investigación del **29-09-2026**. La fecha aportada por el propietario es **31-10-2026**, con **10 h semanales confirmadas por integrante**. La referencia Specter se adaptó al nombre y calendario vigentes; no se ejecuta como una instrucción autónoma.

## Producto e investigación

- [PRD](product/prd.md), [alcance](product/scope.md), [requisitos y trazabilidad](product/requirements.md).
- [Alternativas](research/landscape.md), [OSINT abierta](research/open-osint-selection.md), [correlación investigada](research/recon-correlation-2026-09.md), [factibilidad](research/feasibility.md), [fuentes](research/sources.json).

## Diseño y seguridad

- [Arquitectura C4 vigente](architecture/overview.md), [arquitectura web propuesta](architecture/proposed-web.md), [SOLID y dependencias](architecture/solid.md).
- [Guía e índice de ADRs](adr/README.md): [lenguaje](adr/0001-core-language.md), [grafo](adr/0002-graph.md), [ingestión](adr/0003-ingestion.md), [persistencia](adr/0004-persistence.md), [web](adr/0005-web-events.md), [Burp](adr/0006-burp-boundary.md), [visualización](adr/0007-graph-ui.md), [plataforma propuesta](adr/0008-web-platform.md), [correlación propuesta](adr/0009-evidence-correlation.md), [fuentes propuestas](adr/0010-source-integration.md).
- [Contratos](contracts/README.md), [schema vigente](contracts/snapshot.schema.json), [API web en discusión](contracts/web-api-proposal.md), [API loopback histórica](contracts/api.md) y [eventos diferidos](contracts/events.md).
- [Amenazas](security/threat-model.md), [matriz CWE](security/cwe-controls.md), [scope y cancelación](security/scope.md), [datos](security/data-policy.md).

## Ejecución

- [Desarrollo](development.md), [calidad](testing/strategy.md), [benchmark](testing/benchmark.md), [demo](demo/runbook.md).
- [Roadmap](plan/roadmap.md), [Product Backlog](plan/backlog.md), [Daily Scrum por persona](plan/daily-scrum.md), [equipo](plan/team.md), [sprint 0 histórico](plan/sprint-0.md), [Burp posterior](plan/burp-roadmap.md).
- [BApp checklist](bapp/acceptance.md), [submission borrador](bapp/submission-draft.md), [pruebas manuales](bapp/manual-tests.md).
- [Estándares profesionales](governance/standards.md), [commits y PRs](governance/commits-and-prs.md), [CI y CodeQL](governance/ci-security.md), [licencias](governance/licenses.md), [releases](governance/releases.md).
- [Progreso](process/progress.md), [skills utilizadas](process/skills-usage.md), [decisiones pendientes](process/open-decisions.md), [informe de verificación](process/verification.md).
