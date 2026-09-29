# Registro de trabajo

## 29-09-2026 — Preparación de CHEF

Inventario: checkout main vacío con .git, sin código ni trabajo ajeno. Cuenta GitHub CyberStill-GmbH verificada. gh CLI ausente; git credential manager disponible. Node24 y Python disponibles; JRE8 no permite compilar Montoya.

Objetivo: repositorio público profesional, investigación verificable, separación SOLID, contratos y plan realista; docs branch y merge posterior a main autorizado por propietario.

Evidencia: documento Specter leído como antecedente; fuentes oficiales de PortSwigger/Nmap/Amass/SpiderFoot/Uncover/Cytoscape/NIST/OWASP. Se registran claims/limitaciones, sin experimentos fabricados.

Decisiones: TypeScript para core offline; Java/Montoya autónoma como spike; React/TS previsto; evidencia/identidad conservadora; sin red/shell. Deadline31-10 acorta calendario a32 días y ~102.9h netas bajo10h/persona.

Archivos: README/AGENTS/CONTRIBUTING/SECURITY/LICENSE; docs producto/research/architecture/ADRs/contracts/security/testing/plan/bapp/governance/demo; packages core/adapters y CLI; fixtures/tests/scripts; GitHub CI/templates y skills locales. Véase [verificación](verification.md) para resultados finales.

Incidencias resueltas durante preparación: instalación de herramientas npm requirió npm.cmd; TypeScript6 necesitó types node explícito; auditoría encontró transitiva lodash-es del tooling Mermaid, corregida con actualización compatible del lockfile. No usar estos fallos intermedios como estado final.

Pendiente independiente de acceso externo: frontend/API/worker/persistencia y funciones BApp completas son backlog, no implementación fingida. Cargar el JAR y probarlo dentro de Burp requiere entorno que no está verificado. Siguiente acción del equipo: adoptar contratos y W01 con grafo/lista de snapshot.
