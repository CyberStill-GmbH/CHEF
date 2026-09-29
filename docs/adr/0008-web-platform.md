# ADR 0008: Plataforma web TypeScript y PostgreSQL/Prisma

Estado: **rechazado como propuesta integral TypeScript por nueva decisión del propietario**; conservar como comparación histórica. Fecha: 29-09-2026. La propuesta vigente está en [ADR 0011](0011-node-go-boundary.md): Node.js/Prisma/OAuth para plataforma y Go para correlación. No altera los ADRs aceptados del CLI.

## Contexto

El incremento actual es CLI Node/TypeScript que procesa Nmap XML y exporta `Snapshot 1.0.0`; no hay servidor, DB, OAuth ni UI. El propietario pidió web alojable para equipos antes de BApp, React/TypeScript, PostgreSQL/Prisma, GitHub OAuth y acceso a Vercel/Railway. Quedan unas 103 h netas hasta el 31-10; una web privada completa excede la estimación y la fuente pasiva no está confirmada. El dominio debe seguir sin frameworks/IO y el parser estable.

## Impulsores y alternativas

Se priorizan reutilización del núcleo probado, una sola semántica de identidad, capacidad de revisión por tres personas, aislamiento de proyectos, migraciones auditables y operación simple. **A, API TypeScript/Node:** llama a `ImportEvidence` y usa Prisma/PostgreSQL tras puertos. **B, API Go + core TS por proceso:** mantiene Prisma en servicio TS o requiere otro ORM; añade contrato/operación entre procesos. **C, reescribir core en Go:** abandona parte del trabajo probado, duplica pruebas y aumenta riesgo de drift. [Prisma documenta Node/TypeScript + PostgreSQL](https://www.prisma.io/docs/prisma-orm/quickstart/postgresql); el [cliente Go declara deprecación](https://github.com/prisma/prisma-client-go). Go podría ser razonable tras medición concreta de throughput/operación, no por una expectativa de escala sin benchmark.

## Decisión propuesta

Elegir **A**: monolito modular TypeScript/Node para API y casos de uso, React/TypeScript para presentación, PostgreSQL con Prisma en adaptador. Mantener CLI como interfaz de prueba/importación y `Snapshot 1.0.0` como exportación; no usarlo como tabla ni portar parser al browser. GitHub OAuth se termina en backend; CHEF autoriza cada acción por membresía/rol de proyecto. Vercel para React y Railway para API/DB son destinos candidatos, no despliegues aprobados. No introducir SSE, microservicios ni cola hasta demostrar necesidad de latencia/concurrencia.

## Consecuencias, riesgo y migración

Ventaja: una implementación de reglas/IDs, puertos de prueba, contratación de equipo simple. Coste: API Node debe aislar trabajo pesado de parsing y gestionar límites, sesiones, secretos, migraciones y backups. PostgreSQL requiere constraints/índices por proyecto y transacciones atómicas; Prisma no debe filtrar a dominio/DTO. Separación entre usuarios es control de aplicación; RLS es defensa adicional con salvedad de propietario de tabla. [CWE-862](https://cwe.mitre.org/data/definitions/862.html) y [CWE-639](https://cwe.mitre.org/data/definitions/639.html) son riesgos de autorización a probar. Si Go pasa a ser requisito, abrir ADR sustituto con decisión ORM/puente, estimación y conformance frente al core TS.

Migración propuesta: (1) contrato API/observación y puerto de repositorio; (2) adaptación Prisma/DB en proyecto local; (3) API import/read/export con tests negativos; (4) React; (5) OAuth y despliegue privado con aislamiento/backup probado. No se afirma que la secuencia quepa en octubre: [Scrum](../proposals/2026-09-scrum-to-oct31.md) registra el desfase.

## Verificación para aceptar e implementar

Given XML/allowlist del fixture, When se importa por API y por CLI con reloj fijo, Then IDs/relaciones/evidencia coinciden y el CLI sigue estable. Given usuarios A/B en proyectos distintos, When B consulta/importa/exporta IDs de A, Then no recibe datos ni confirma existencia. Given XML hostile o scope inválido, When falla importación, Then no queda run parcial. La aceptación del ADR necesita review humano de contrato/operación; el Done de código requiere `npm run check`, DB de prueba, migración/rollback, sesión y backups verificados por separado.

Fuentes consultadas el 29-09-2026 en [registro](../research/sources.json): Prisma/PostgreSQL, Prisma Go, GitHub OAuth, PostgreSQL RLS y documentación Vercel/Railway. [C4 propuesto](../architecture/proposed-web.md) muestra límites.
