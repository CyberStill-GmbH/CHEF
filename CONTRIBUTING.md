# Contribuir a CHEF

CHEF es un proyecto del Centro Cultural de Ciberseguridad para reconocimiento **autorizado** con evidencia explicable. El núcleo/CLI TypeScript offline y el explorador React de snapshots están implementados; `packages/rust-core` contiene el primer portado con paridad inicial de IDs. La dirección de producto aceptada es Tauri v2 + Rust + SurrealDB local; la migración completa requiere revisión e implementación. Antes de empezar, lee [alcance](docs/product/scope.md), [requisitos de escritorio](docs/product/desktop-requirements.md), [arquitectura actual](docs/architecture/overview.md), [arquitectura Tauri/Rust](docs/architecture/desktop-tauri.md), [ADRs](docs/adr/README.md) y [backlog de migración](docs/plan/backlog-desktop.md). Las reglas adicionales están en [AGENTS.md](AGENTS.md).

## Recorrido de una contribución

1. Elige una historia `listo` del backlog o abre una issue con problema y criterio Dado/Cuando/Entonces. Comprueba que fuente/permiso, contrato, responsable y revisor estén definidos. Si el cambio afecta una decisión difícil de revertir, propone un ADR antes de implementar.
2. Parte de la rama destino vigente indicada por el backlog o mantenedor, actualízala y crea una rama propia. Inspecciona `git status` y conserva trabajo ajeno. Usa el [formato de rama/commit](docs/governance/commits-and-prs.md), con commits atómicos y un cambio revisable por PR.
3. Implementa respetando capas: dominio Rust sin Tauri/IO, casos de uso por puertos, adaptadores de infraestructura y composición en Tauri. React no recalcula identidad ni consulta la base directamente. Conserva IDs, observaciones, evidencias, control negativo y la CLI TypeScript como referencia de migración. No cambies `Snapshot 1.0.0` silenciosamente.
4. Añade prueba positiva **y de fallo relevante**. Para port Rust, compara golden TypeScript; para IPC valida versión, límites, orden y cancelación; para persistencia revisa cierre/reinicio y referencias. Para UI comprueba teclado, estados y texto importado seguro. Para fuentes remotas, exige permiso y fixture; para escaneo activo, scope escrito y laboratorio aprobado.
5. Ejecuta `npm ci` y `npm run check`; este comando verifica también formato, Clippy y pruebas Rust. Si tocas Montoya, compila `mvn -B -f extensions/burp/pom.xml clean verify` y registra aparte las pruebas manuales Burp realizadas/pendientes. Prepara la [plantilla de PR](.github/PULL_REQUEST_TEMPLATE.md), enlaza issue/ADR, muestra resultados y solicita revisión de otra persona.
6. Tras aprobación independiente, conversaciones resueltas y CI verde, el maintainer integra la PR con **Squash and merge**; el título final sigue `tipo(área): resultado concreto`. No conviertas “propuesto” o “compila” en “validado en navegador/Burp/desplegado”. El maintainer verifica el resultado en la rama destino.

## Seguridad, datos y fuentes

No ejecutar descubrimiento activo sin alcance y autorización escritos. Los fixtures son sintéticos/offline; un archivo Nmap importado no amplía el alcance. No subir datos de clientes, exportaciones, tokens ni secretos. Guardar preferencias en TOML y secretos en un almacén seguro del sistema operativo. Una fuente pasiva o envío DefectDojo requiere permiso, vista previa y consentimiento explícito. Observaciones históricas o coincidencias débiles se muestran como candidatas, nunca vulnerabilidades confirmadas. Los riesgos y controles están en [seguridad](docs/security/cwe-controls.md); reportar vulnerabilidades por [SECURITY](SECURITY.md).

## Equipo y decisiones

César lleva el núcleo Rust/IPC; Diego frontend y xterm.js; Jhojan persistencia, fixtures y conformidad. Una persona diferente revisa cada PR, según [equipo](docs/plan/team.md). Los ADRs se abren con la [plantilla](docs/adr/TEMPLATE.md) y se aceptan por revisión documentada; una propuesta fusionada no significa código implementado. El [Daily Scrum](docs/plan/daily-scrum.md) y el [plan vigente](docs/plan/scrum-actual.md) exponen dependencias y capacidad.

Al contribuir código original, confirma que puedes licenciarlo bajo [MIT](LICENSE). Antes de reutilizar código/fixtures de Nmap, PortSwigger o terceros, revisa [licencias](docs/governance/licenses.md) y atribución.
