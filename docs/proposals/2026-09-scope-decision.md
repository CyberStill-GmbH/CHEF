# Propuesta para revisión: CHEF web antes de Burp

Fecha: 29-09-2026. **Propuesta, no ADR aceptado ni función implementada.** Esta rama deja intactos el núcleo, la CLI y los contratos publicados. El propietario actualizó la visión: React/TypeScript, PostgreSQL con Prisma ORM, acceso GitHub OAuth y correlación activo–pasivo como núcleo de una aplicación web alojable y multiusuario. La extensión Burp vendrá después. Esto sustituye el antiguo acuerdo de presentar la CLI como experiencia principal; la demostración offline sigue siendo prueba de regresión útil, no la meta de producto.

## 1. Decisión de alcance y coste

| Corte                                            | Lo que se vería                                                                            | Estimación incremental | Riesgo                                             |
| ------------------------------------------------ | ------------------------------------------------------------------------------------------ | ---------------------: | -------------------------------------------------- |
| A. Web vertical local                            | Importar Nmap y un fixture pasivo, correlación explicada en React, persistencia PostgreSQL |              110–160 h | Excede las ~103 h netas disponibles hasta el 31-10 |
| B. Web privada desplegada                        | A más GitHub OAuth, roles/proyectos, despliegue, recuperación y aislamiento probado        |              +70–120 h | Seguridad y operación sin hosting definido         |
| C. Web pública multiusuario con trabajos activos | B más cola, workers aislados, scope aprobado, cuotas, egress, auditoría y fuentes vivas    |       +120–220 h o más | No hay autorización ni fuente/entorno confirmados  |

Son estimaciones de planificación, no mediciones. La presentación sigue siendo el 31-10-2026; César, Diego y Jhojan disponen de 10 h semanales cada uno. Del 29-09 al 30-10 hay unas 137 h brutas; reservar 25 % deja ~103 h netas. **El propietario eligió B, prototipo desplegado para el equipo y demostrable en vivo**, con el recorrido de crear proyecto, importar Nmap, consultar pasivo, revisar relaciones/evidencia y exportar. Esta meta no cabe en la estimación actual: el equipo debe negociar recorte, capacidad o fecha sin llamar terminadas a piezas ausentes. A puede reducirse a Nmap + fixture pasivo sintético, pero así no demuestra OSINT vivo ni ahorro real.

## 2. Estado real y arquitectura propuesta

**Implementado y probado:** caso de uso `ImportEvidence` en TypeScript/Node, parser Nmap XML acotado, scope literal, identidad determinista, deduplicación conservadora, evidencia y exportación `Snapshot 1.0.0` por CLI; 11 pruebas TypeScript. **Spike separado:** Java/Montoya normaliza una selección HTTP, sin correlación con Nmap ni prueba manual dentro de Burp. **No implementado:** web, API, PostgreSQL/Prisma, OAuth, sesiones, aislamiento de proyectos, fuente pasiva en el core, escáner activo, actualización en tiempo real ni ahorro medido.

```text
React/TypeScript ── HTTPS/API ── aplicación Node/TypeScript ── puertos
   grafo + lista                │                 ├─ PostgreSQL/Prisma: proyectos, runs, observaciones
   evidencia, reglas            │                 ├─ importador Nmap existente
   no decide identidad          └─ casos de uso   ├─ adaptador pasivo autorizado
                                                 └─ futuro worker activo con scope aprobado
                      Snapshot 1.0.0: exportación/intercambio, no tablas ORM
```

El dominio permanece sin filesystem, red, React, Prisma ni Montoya; application depende de puertos y adaptadores viven en infraestructura. El servicio Node llama al caso de uso existente: no portar el parser al navegador ni convertir Prisma en modelo de dominio. La CLI queda como herramienta de ingestión/regresión. PostgreSQL guarda datos por proyecto y ejecución; Prisma es adaptador de persistencia, no contrato público. El snapshot JSON versionado conserva interoperabilidad; un lector externo debe validar versión, IDs únicos y referencias antes de importar. OAuth con GitHub identifica al usuario; la autorización a proyectos y operaciones la decide CHEF en servidor. Cada consulta/mutación debe estar acotada al proyecto, con pruebas negativas entre tenants. [PostgreSQL documenta RLS](https://www.postgresql.org/docs/current/ddl-rowsecurity.html) como defensa adicional, con salvedades de propietario de tabla; no sustituye la autorización de aplicación. [GitHub documenta OAuth](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps) y sus [prácticas de seguridad](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/best-practices-for-creating-an-oauth-app). Fijar una versión estable probada de [Prisma/PostgreSQL](https://www.prisma.io/docs/prisma-orm/quickstart/postgresql) al implementar.

**Recomendación técnica:** backend TypeScript/Node. Reutiliza el caso de uso, pruebas, reglas e identidad actuales y usa Prisma directamente. Go sigue siendo viable como servicio separado si una medición futura exige otro perfil de concurrencia, pero hoy obliga a puente o reescritura; [Prisma Client Go declara deprecación](https://github.com/prisma/prisma-client-go) y no seguirá Prisma v7+. No repartir el motor entre Go y TS solo por preferencia de lenguaje. Registrar la decisión final en ADR 0001/0004 tras revisión.

## 3. Diferencial propuesto y límites

**Hipótesis:** un libro de evidencia temporal y explicable ahorra el trabajo de limpiar Nmap y reconciliar fuentes. Cada observación mantiene fuente, locator, hash, tiempo, alcance y semántica. La normalización clasifica duplicados/ruido en un resumen reversible: conserva recuentos y motivos mientras la vista prioriza servicio alcanzable, relación corroborada, contradicción y novedad. Los estados `open`, `filtered` y `open|filtered` siguen distintos; [Nmap explica por qué](https://nmap.org/book/man-port-scanning-basics.html). Solo claves fuertes verificables permiten fusión automática; hostname, IP compartida o banner generan candidatos, nunca propiedad ni vulnerabilidad confirmadas. Mostrar regla y evidencia a favor/en contra, momento de observación y frescura. Permitir cuestionar, separar o confirmar una relación sin destruir la fuente original.

El modelo de procedencia se inspira en [W3C PROV-O](https://www.w3.org/TR/prov-o/). El enlace probabilístico de [Fellegi–Sunter](https://www.tandfonline.com/doi/full/10.1080/01621459.1969.10501049) y [Splink](https://moj-analytical-services.github.io/splink/topic_guides/theory/probabilistic_vs_deterministic.html) es alternativa para candidatos cuando exista corpus etiquetado; no asignar puntuaciones ni entrenar sin calibración. La primera implementación debe ser determinista, con reglas versionadas, claves tipadas, bloqueo por scope y controles negativos. “Superficie en tiempo real” requiere medir latencia desde evidencia nueva hasta vista, caducidad por fuente y fallos de actualización; hasta entonces es superficie **actualizada por ejecución**.

Medición propuesta: mismo caso de laboratorio con XML Nmap y una fuente pasiva disponible; tareas de localizar servicio, explicar enlace, detectar falso enlace y exportar evidencia. Comparar CHEF con proceso manual en orden contrabalanceado; registrar tiempo, aciertos, falsos enlaces y observaciones perdidas. Criterio inicial propuesto: reducir mediana de tiempo ≥30 % sin aumentar errores en ≥5 evaluadores; no es resultado observado. Comparar funciones de [Amass](https://github.com/owasp-amass/amass), [SpiderFoot](https://github.com/smicallef/spiderfoot), [Uncover](https://github.com/projectdiscovery/uncover) y BApps antes de afirmar novedad. El diferencial se busca en explicación de fusión y limpieza trazable dentro del flujo, no en importar XML o dibujar un grafo.

## 4. Decisiones aún abiertas

1. Recorte/capacidad para hacer viable el prototipo privado en octubre; el recorrido objetivo ya está confirmado, no su factibilidad.
2. Primera fuente pasiva con acceso y permiso, además del XML Nmap del repo. El envelope HTTP disponible es fixture de conformidad, **no** una ingestión pasiva integrada. CT/RDAP son candidatos, no acceso operativo confirmado.
3. Activo: **solo importación de resultados autorizados en esta fase**, sin lanzar escaneos desde CHEF. El entorno de demostración usa datos autorizados; fuentes/targets concretos aún deben registrarse.
4. Relaciones y etiquetas de referencia, y umbral de falso enlace aceptable. El fixture actual cubre Nmap/control negativo, no correlación multifuente.
5. Hay acceso a Vercel y Railway como candidatos; faltan presupuesto, retención, clasificación de datos, backups, administradores y usuarios piloto.
6. Responsables/reviewers por área dentro de 10 h semanales; [equipo](../plan/team.md) recoge roles actuales, pero el nuevo alcance requiere refinamiento.

Véanse [investigación](../research/recon-correlation-2026-09.md), [Scrum](2026-09-scrum-to-oct31.md), [issues](2026-09-issues.md) y [brechas BApp](2026-09-bapp-gaps.md). Tras decidir, actualizar PRD, scope, ADR 0001/0004/0005/0007, contratos API/snapshot si hiciera falta, roadmap, backlog, arquitectura, amenazas y demo en PRs pequeños. Esta propuesta no reemplaza decisiones aceptadas por mera publicación.
