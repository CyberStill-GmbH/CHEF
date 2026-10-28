# Documentación de CHEF

La dirección de producto actual es la aplicación de escritorio Tauri/Rust del [ADR 0012](adr/0012-escritorio-tauri-rust.md), aceptada por el propietario; la revisión técnica y la implementación siguen pendientes. El [PRD de escritorio](product/desktop-prd.md), los [requisitos](product/desktop-requirements.md), la [arquitectura](architecture/desktop-tauri.md), el [backlog de migración](plan/backlog-desktop.md) y el [plan Scrum vigente](plan/scrum-actual.md) documentan el trabajo. Los documentos web de septiembre se conservan como antecedentes, no como plan activo.

Los documentos registran decisiones y evidencia con su fecha. La presentación objetivo aportada por el propietario es **31-10-2026**, con una referencia de hasta **10 h semanales por integrante**. La referencia Specter se adaptó al nombre y calendario vigentes; no se ejecuta como una instrucción autónoma.

## Producto e investigación

- [PRD de escritorio](product/desktop-prd.md), [alcance](product/scope.md), [requisitos de escritorio](product/desktop-requirements.md), [requisitos web históricos](product/requirements.md).
- [Alternativas](research/landscape.md), [OSINT abierta](research/open-osint-selection.md), [correlación investigada](research/recon-correlation-2026-09.md), [factibilidad](research/feasibility.md), [fuentes](research/sources.json).

## Diseño y seguridad

- [Arquitectura C4 del código actual](architecture/overview.md), [arquitectura Tauri/Rust propuesta](architecture/desktop-tauri.md), [arquitectura web anterior](architecture/proposed-web.md), [SOLID y dependencias](architecture/solid.md).
- [Persistencia local, grafo y búsqueda semántica](architecture/persistencia-local.md), [auditoría de la fuente](migration/auditoria-fuente.md), [plan de migración del backend a Rust](migration/plan-migracion-rust.md) e [inventario técnico](migration/inventario-migracion.csv).
- [Guía e índice de ADRs](adr/README.md): [lenguaje](adr/0001-core-language.md), [grafo](adr/0002-graph.md), [ingestión](adr/0003-ingestion.md), [persistencia](adr/0004-persistence.md), [web](adr/0005-web-events.md), [Burp](adr/0006-burp-boundary.md), [visualización](adr/0007-graph-ui.md), [plataforma web histórica](adr/0008-web-platform.md), [correlación propuesta](adr/0009-evidence-correlation.md), [fuentes propuestas](adr/0010-source-integration.md), [límite Node-Go retirado](adr/0011-node-go-boundary.md) y [escritorio Tauri/Rust propuesto](adr/0012-escritorio-tauri-rust.md).
- [Contratos](contracts/README.md), [schema vigente](contracts/snapshot.schema.json), [API web en discusión](contracts/web-api-proposal.md), [API loopback histórica](contracts/api.md) y [eventos diferidos](contracts/events.md).
- [Amenazas](security/threat-model.md), [matriz CWE](security/cwe-controls.md), [scope y cancelación](security/scope.md), [datos](security/data-policy.md).

## Ejecución

- [Desarrollo](development.md), [calidad](testing/strategy.md), [benchmark](testing/benchmark.md), [demo](demo/runbook.md).
- [Hoja de ruta](plan/roadmap.md), [backlog de migración](plan/backlog-desktop.md), [Scrum vigente con reparto para tres personas](plan/scrum-actual.md), [Daily Scrum](plan/daily-scrum.md), [equipo](plan/team.md), [backlog web anterior](plan/backlog.md), [Burp posterior](plan/burp-roadmap.md).
- [BApp checklist](bapp/acceptance.md), [submission borrador](bapp/submission-draft.md), [pruebas manuales](bapp/manual-tests.md).
- [Estándares profesionales](governance/standards.md), [commits y PRs](governance/commits-and-prs.md), [CI y CodeQL](governance/ci-security.md), [licencias](governance/licenses.md), [releases](governance/releases.md).
- [Progreso](process/progress.md), [skills utilizadas](process/skills-usage.md), [decisiones pendientes](process/open-decisions.md), [informe de verificación](process/verification.md).
