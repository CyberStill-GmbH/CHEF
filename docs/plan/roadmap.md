# Roadmap de CHEF

Actualizado 29-09-2026. **La web es el producto primero; Burp viene después.** La presentación objetivo es 31-10-2026. El propietario pidió prototipo privado desplegado para el equipo, con GitHub OAuth, PostgreSQL/Prisma, Nmap autorizado, una fuente OSINT pasiva, correlación explicada, mapa y exportación. El [análisis de capacidad](../research/feasibility.md) advierte que este alcance completo supera 10 h/semana por cada una de tres personas. No se transforma un objetivo deseado en un compromiso sin refinement.

## Etapa 0 · base ya existente

Núcleo/CLI TypeScript offline, Nmap XML subset, scope, IDs deterministas, evidencia y snapshot versionado; tests y CI. El spike Montoya Java es aparte y su uso en Burp requiere prueba manual. Esta base se conserva en todas las etapas mediante fixtures/golden y `npm run check`.

## Etapa 1 · vertical web de octubre, sujeta a capacidad

1. **29-09–05-10:** decidir stack/contrato, fuente pasiva/permiso y corte viable. Gate: sin fuente/dataset autorizado se rotula fixture sintético y se retira claim de OSINT viva.
2. **06-10–12-10:** API Node que reutiliza core, proyecto y PostgreSQL/Prisma detrás de puerto, importación Nmap desde React. Gate: CLI sin regresión, dos proyectos aislados, salida atómica.
3. **13-10–19-10:** una fuente pasiva y una relación fuerte/candidata explicada; grafo/lista y panel de evidencia. Gate: control negativo, fechas, regla y dos procedencias.
4. **20-10–30-10:** export, UX/accesibilidad, GitHub OAuth, despliegue privado y ensayo comparativo **solo según capacidad real**. Gate: no llamar multiusuario al prototipo sin sesiones/IDOR/backup/migración probados. Freeze 30-10 y presentación 31-10 con estado honesto.

La [pauta Scrum detallada](../proposals/2026-09-scrum-to-oct31.md) muestra objetivo, tareas de cada persona, estimaciones, gates y recortes. La [pauta Daily](daily-scrum.md) convierte cada sprint en trabajo coordinado. El fallback técnico del núcleo offline permanece disponible; mostrarlo no cumple por sí solo la nueva meta web.

## Etapa 2 · producto web completo, sin fecha fijada

Conectar fuente pasiva real con licencia/permiso, completar proyectos/OAuth y operación privada, historial temporal, refresco con latencia y frescura medida, export JSON e imagen segura, observabilidad y evaluación con pentesters. Añadir conectores OSINT por valor/fiabilidad, uno por PR y contrato. El mapa “en tiempo real” se reclama solo con SLA observado; antes se muestra “última actualización”. Un worker de reconocimiento activo se evalúa únicamente tras nueva autorización/ADR/aislamiento.

## Etapa 3 · integración Burp/BApp, posterior

Elegir BApp autónoma o cliente del servicio, mantener conformance Java/TypeScript, probar manualmente carga/unload/performance y demostrar ahorro dentro de Burp. Revisar [criterios PortSwigger](https://portswigger.net/burp/documentation/desktop/extend-burp/extensions/creating/bapp-store-acceptance-criteria) en fecha de postulación. No fijar entrega ni prometer aceptación antes de cerrar el producto web y validar el valor diferencial.

Cada gate puede reordenar Product Backlog con evidencia; los criterios de Done, scope, integridad y seguridad no se recortan. Decisiones y cambios viven en [ADRs](../adr/README.md), [backlog](backlog.md) y actas/PRs.
