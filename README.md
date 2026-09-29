# CHEF

**Evidence-backed attack surface correlation for authorized assessments.**

CHEF convierte evidencia dispersa en relaciones que se pueden inspeccionar y reproducir. Cada activo y relación enlaza a observaciones y a la huella del archivo que las originó. Una ruta de exposición es una hipótesis para orientar la revisión; un puerto abierto no demuestra una vulnerabilidad.

Proyecto estudiantil para presentación el **31 de octubre de 2026**. La postulación a BApp Store es una fase posterior y su aceptación depende de PortSwigger.

## Estado real

- Implementado: núcleo TypeScript con separación SOLID, importación Nmap XML sin red, alcance explícito, deduplicación, procedencia, grafo JSON, exportación por CLI, fixtures adversariales y benchmark sintético.
- Spike separado: adaptador Java/Montoya para selección de requests y observaciones de endpoint; requiere validación manual en Burp. No es una BApp terminada.
- Planificado: explorador React + TypeScript, API/eventos, persistencia SQLite, ingestión HTTP/DNS y correlación entre fuentes. Los directorios reservados contienen contratos de responsabilidad, no servicios ficticios.

## Ejecutar

Requiere Node.js 24 LTS y npm. No requiere Nmap, Burp ni una cuenta de terceros para la demo del núcleo.

```sh
npm ci
npm run check
npm run demo
npm run benchmark
```

Exportar JSON limpio después de compilar:

```sh
npm run build
node dist/apps/cli/src/main.js fixtures/nmap/lab.xml fixtures/scope/lab.json > chef.json
```

El fixture es sintético y usa direcciones de loopback. CHEF no ejecuta escaneos. Las evidencias reales y los secretos no deben entrar en Git; las exportaciones pueden contener información sensible de los activos.

## Recorrido para evaluar el proyecto

1. [Investigación y diferenciación](docs/research/landscape.md).
2. [Producto y requisitos verificables](docs/product/prd.md).
3. [Arquitectura y SOLID](docs/architecture/overview.md).
4. [Contratos versionados](docs/contracts/README.md).
5. [Plan hasta la presentación](docs/plan/roadmap.md).
6. [Criterios y preparación para BApp Store](docs/bapp/acceptance.md).
7. [Demo de cinco minutos](docs/demo/runbook.md).

Consulta el [índice completo](docs/README.md), la [guía local](docs/development.md) y [CONTRIBUTING](CONTRIBUTING.md). Las reglas de colaboradores están en [AGENTS.md](AGENTS.md).

## Estructura

```text
packages/domain       modelo, invariantes y correlación pura
packages/application  casos de uso; depende de puertos del dominio
packages/adapters     Nmap, alcance, SHA-256 y validación de contratos
apps/cli              composición y acceso a archivos locales
apps/api              frontera prevista de API; todavía sin servidor
apps/web              contrato previsto para React/TypeScript
extensions/burp       spike autónomo Java/Montoya
fixtures              datos sintéticos y etiquetas del benchmark
docs                  investigación, decisiones, seguridad y planificación
.agents/skills        guías de trabajo específicas de CHEF
```

Licencia MIT para el código original de CHEF. [Dependencias y límites de redistribución](docs/governance/licenses.md). Sin afiliación ni aprobación de PortSwigger, Nmap o UTEC.
