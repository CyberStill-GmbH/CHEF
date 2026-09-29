# Alcance de CHEF

Actualizado 29-09-2026. **Visión confirmada por el propietario:** producto web alojable y multiusuario del Centro Cultural de Ciberseguridad, React/TypeScript, **API Node.js/TypeScript con PostgreSQL/Prisma y GitHub OAuth**, y **motor principal de correlación en Go** para reconocimiento activo importado y OSINT pasiva; mapa interactivo y exportación, Burp después. La [frontera Node–Go](../adr/0011-node-go-boundary.md) está propuesta para revisión técnica; ninguno de esos componentes web/Go está implementado. La presentación objetivo es el **31-10-2026**, con 10 h semanales confirmadas para César, Diego y Jhojan.

## Resultado de usuario

Un pentester autorizado debe poder crear/abrir proyecto, importar Nmap XML autorizado, consultar una fuente pasiva autorizada, revisar servicios y relaciones con fechas/procedencia, distinguir observaciones de candidatos y descargar datos de evidencia. El producto no declara vulnerabilidad confirmada por un puerto o relación. “Limpieza” significa vista priorizada y deduplicación conservadora **con recuento y causa**, no eliminación opaca de registros. “Tiempo real” es objetivo posterior sujeto a latencia/frescura medibles.

## Implementado / propuesto / bloqueado

- **Implementado:** CLI offline Node/TypeScript, Nmap XML subset, scope literal, IDs estables, evidencia y `Snapshot 1.0.0`; 11 pruebas core. El spike Montoya se prueba por separado y no forma parte del flujo web.
- **Propuesto para primera vertical:** API Node que reutilice el parser/caso de uso Nmap existente, PostgreSQL/Prisma por puerto, proyectos, React con grafo/lista y panel de evidencia, adaptadores OSINT pasivos y motor Go tras contrato versionado para correlación explicada y exportación. OAuth/despliegue privado son objetivo del prototipo del equipo, no hechos actuales.
- **Bloqueado por dato externo:** se seleccionaron [fuentes pasivas prioritarias](../research/open-osint-selection.md), pero faltan permiso para consultar objetivos, aceptación de términos, prueba de acceso y dataset que corresponda a Nmap. El fixture actual usa loopback y `.invalid`; ningún proveedor público devolverá honestamente esa pareja. No se simulará OSINT viva como fuente real.

El prototipo deseado para octubre es **privado, desplegado para el equipo y demostrable en vivo**. El [análisis de capacidad](../proposals/2026-09-scrum-to-oct31.md) indica que el recorrido completo supera las ~103 h netas disponibles: priorizar y reestimar en gate 05-10, no prometer que todo estará Done. Si una vertical se corta a datos sintéticos o web local, registrar la brecha frente al objetivo original. Mantener una demo de regresión offline reproducible mientras se construye la web.

## Fases posteriores, sin fecha prometida

Primero completar fuente pasiva real y medición de ahorro; luego añadir conectores uno a uno con términos, cuota y tests; después historial temporal/actualización medida y exportaciones de informe. Un worker de reconocimiento activo requiere una decisión nueva con autorización escrita, scope, límites de red, egress, cuotas, cancelación y auditoría. La BApp Java/Montoya se diseña después del producto web con criterio de utilidad dentro de Burp y fixtures de conformidad; compilar el JAR no acredita aceptación en BApp Store.

## Exclusiones y reglas no negociables

Sin explotación automática, brute force, C2, evasión, inyección, shell ni ofuscación. **CHEF no lanza escaneos en esta fase**; solo importa resultados autorizados. No enviar dominios o datos de clientes a terceros por defecto. No ampliar scope por un archivo/importación. No fusionar propiedad de hosts por hostname, IP compartida, banner o certificado. No ocultar `filtered`/`open|filtered` como `open`; no convertir evidencia histórica en estado presente. No mezclar proyectos/usuarios. No publicar datos de clientes, secretos o exportaciones en Git.

Cada cambio de alcance registra requisito, fuente/permiso, persona responsable, horas, pantalla objetivo, contrato/ADR afectado y efecto en el 31-10. La [propuesta de decisiones](../proposals/2026-09-scope-decision.md), [requisitos](requirements.md), [arquitectura](../architecture/overview.md) y [OSINT abierta](../research/open-osint-selection.md) detallan los gates.
