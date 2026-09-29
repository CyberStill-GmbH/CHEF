# Contribuir a CHEF

CHEF es un proyecto del Centro Cultural de Ciberseguridad para reconocimiento **autorizado** con evidencia explicable. El incremento actual es un núcleo/CLI TypeScript offline; la web React + PostgreSQL/Prisma es el producto prioritario en construcción y Burp viene después. Antes de empezar, lee [alcance](docs/product/scope.md), [requisitos](docs/product/requirements.md), [C4 actual/propuesto](docs/architecture/overview.md), [ADRs](docs/adr/README.md) y [Product Backlog](docs/plan/backlog.md). Las reglas de colaboración adicionales están en [AGENTS.md](AGENTS.md).

## Recorrido de una contribución

1. Elige una historia `ready` del backlog o abre issue con problema y Given/When/Then. Comprueba que fuente/permiso, contrato, owner y reviewer están definidos. Si el cambio afecta una decisión irreversible, propone un ADR antes de implementar.
2. Parte de `main` actualizado en una rama propia. Inspecciona `git status` y conserva trabajo ajeno. Usa el [formato de rama/commit](docs/governance/commits-and-prs.md), con un cambio revisable por PR.
3. Implementa respetando capas: dominio puro, application por puertos, adaptadores de infraestructura y composición en apps. React no recalcula identidad; Prisma no entra en dominio. Conserva IDs, observaciones, evidencias, control negativo y la CLI. No cambies `Snapshot 1.0.0` silenciosamente.
4. Añade prueba de éxito **y de fallo relevante**. Para contrato: schema, ejemplo, prueba y compatibilidad. Para UI: teclado, lista equivalente, estados vacío/error y texto importado seguro. Para multiusuario: prueba dos proyectos/usuarios e IDOR. Para proveedor OSINT: permiso, fixture, cuota/frescura y error sin datos falsos.
5. Ejecuta `npm ci` y `npm run check`; si tocas Montoya, compila `mvn -B -f extensions/burp/pom.xml clean verify` y registra por separado pruebas manuales en Burp realizadas/pendientes. Prepara la [plantilla de PR](.github/PULL_REQUEST_TEMPLATE.md), enlaza issue/ADR, muestra resultados reales y solicita revisión de otra persona.
6. Tras review y CI verde, integra mediante PR. No conviertas “propuesto” o “compila” en “validado en navegador/Burp/desplegado”. El maintainer verifica el resultado en `main`.

## Seguridad, datos y fuentes

No ejecutar descubrimiento activo sin scope y autorización escritos. Los fixtures predeterminados son sintéticos/offline; un archivo Nmap importado no amplía scope. No subir datos de clientes, exportaciones, tokens ni secretos. Una fuente OSINT se integra solo tras verificar licencia/términos, acceso, cuota, fecha y manejo de datos; software abierto no significa datos reutilizables sin condiciones. Observaciones históricas o coincidencias débiles se muestran como candidatas, nunca vulnerabilidades confirmadas. Los riesgos CWE/controles están en [seguridad](docs/security/cwe-controls.md); reportar vulnerabilidades por [SECURITY](SECURITY.md).

## Equipo y decisiones

César coordina backend/correlación y prioridad; Diego frontend/UX; Jhojan ingestión/fixtures/investigación. Una persona diferente revisa cada PR, según [team.md](docs/plan/team.md). Los ADRs se abren con [TEMPLATE](docs/adr/TEMPLATE.md) y se aceptan por revisión documentada; una propuesta fusionada no significa código implementado. La [pauta Daily](docs/plan/daily-scrum.md) y el [Scrum de octubre](docs/proposals/2026-09-scrum-to-oct31.md) exponen dependencias y capacidad.

Al contribuir código original, confirma que puedes licenciarlo bajo [MIT](LICENSE). Antes de reutilizar código/fixtures de Nmap, PortSwigger o terceros, revisa [licencias](docs/governance/licenses.md) y atribución.
