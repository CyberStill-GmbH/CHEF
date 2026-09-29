# ADR 0011: API Node.js, persistencia Prisma y motor de correlación Go

Estado: **propuesto para revisión técnica; selección de lenguajes confirmada por el propietario**. Fecha: 29-09-2026. Responsables: César (API y reglas), Jhojan (fixtures y conformidad), Diego (contrato de explicación). Sustituye la recomendación íntegramente TypeScript del [ADR 0008](0008-web-platform.md) para la web futura. No cambia el [ADR 0001](0001-core-language.md) ni el comportamiento del CLI actual hasta completar una migración verificada.

## Contexto y decisión

El core existente en TypeScript importa Nmap y emite `Snapshot 1.0.0` con IDs estables y evidencia; no hay API, base, OAuth ni motor Go implementados. El propietario eligió **React/TypeScript + API Node.js/TypeScript con GitHub OAuth y PostgreSQL/Prisma + motor principal de correlación en Go**. Burp sigue después. Separar lenguajes agrega contrato y despliegue, pero permite a Node concentrarse en identidad, autorización, ingesta y datos, y a Go concentrarse en comparación, deduplicación y explicaciones reproducibles. No se alega una ventaja de rendimiento sin benchmark.

La API Node valida sesión y membresía de proyecto, aplica scope, recibe XML y consulta OSINT mediante adaptadores autorizados, usa Prisma solo en infraestructura y persiste observaciones. El parser Nmap y CLI TypeScript existentes permanecen estables. Go recibe **observaciones canónicas ya validadas**, scope explícito, reloj fijo y versión de reglas; devuelve afirmaciones de relación, razones, evidencia a favor/en contra y métricas de deduplicación. Go no obtiene credenciales OAuth/OSINT, no consulta proveedores, no accede a PostgreSQL y no decide membresía. React solo presenta los DTO de la API.

## Frontera y transporte

El primer incremento propone un ejecutable Go local invocado por Node sin shell, con JSON versionado por `stdin/stdout` y un trabajo acotado por importación; **la selección exacta de transporte se valida en un spike antes de construir la API**. La API fija tamaño de entrada/salida, tiempo máximo, cancelación, concurrencia y tratamiento de exit codes. `stderr` es diagnóstico sin secretos ni payload. Nunca se persiste un resultado parcial si Go falla. Este corte evita exponer un servicio adicional; empaquetar ambos binarios exige un build reproducible y pruebas en Railway. Si volumen/latencia justifican un servicio Go separado, abrir ADR de transporte y autenticación interna; [Railway documenta red privada](https://docs.railway.com/networking/private-networking/how-it-works), pero no equivale a autorización entre servicios. [Node documenta `execFile`/`spawn`](https://nodejs.org/api/child_process.html); no interpolar entradas en comandos de shell.

El contrato propuesto `CorrelationJob v0` tiene `schemaVersion`, `projectScope`, `ruleSetVersion`, `asOf`, observaciones tipadas con IDs de evidencia, fuente/locator/digest y tiempos; la respuesta tiene assertions tipadas, `ruleId/version`, referencias, estado `observed|inferred|candidate|conflicted`, explicación y recuentos. **No es aún contrato aceptado.** Antes de implementarlo: JSON Schema, ejemplos positivo/negativo, validadores TS/Go, fixtures compartidos y nota de compatibilidad. `Snapshot 1.0.0` no se reinterpreta: un snapshot multifuente requiere otra versión. El límite Go no recibe datos de otros proyectos en el mismo job; Node comprueba pertenencia antes de construirlo.

## Opciones y consecuencias

- **Solo TypeScript:** menor coste de octubre y reutilización directa del core; descartado para el producto futuro por decisión del propietario. Sigue siendo referencia de conformidad, no segundo motor web.
- **Go con PostgreSQL/ORM propio:** duplica autorización, transacciones y modelo de datos; descartado. El [cliente Prisma Go está deprecado](https://github.com/prisma/prisma-client-go), así que Prisma se queda en Node.
- **Node + Go local:** frontera explícita y un único servicio público; coste de empaquetado y ejecución por trabajo. Es la primera hipótesis a probar.
- **Node + servicio Go privado:** permite escalar por separado, pero agrega red, autenticación, despliegue, observabilidad y fallos distribuidos; solo tras medición o restricción operativa real.

El cambio incrementa el esfuerzo frente a la estimación anterior; el [backlog](../plan/backlog.md) y el gate del 05-10 deben reestimarse, sin prometer la web privada completa con ~103 h netas. Mantener **una única implementación autoritativa de las nuevas reglas** en Go y fixtures de conformidad contra el comportamiento TS aceptado. Migrar por reglas, nunca cambiar IDs existentes sin contrato versionado y prueba de compatibilidad. La CLI TS continúa como regresión aun cuando la web invoque Go.

## Criterios para aceptar e implementar

1. Given fixture Nmap actual, When CLI TS corre antes/después, Then `Snapshot 1.0.0`, IDs, evidencia y 11 pruebas siguen iguales.
2. Given job multifuente fijo, When Go corre dos veces y en distinto orden de entrada, Then produce assertions canónicas equivalentes; duplicados conservan recuento y causa.
3. Given hostname/IP compartido, CT histórico o estado `open|filtered`, When correlaciona, Then no confirma propiedad, servicio abierto ni vulnerabilidad; muestra candidato o conflicto con pruebas.
4. Given crash/timeout/salida inválida de Go o referencia rota, When Node procesa el job, Then falla sin publicación parcial y deja error trazable sin payload sensible.
5. Given usuarios/proyectos A y B, When B consulta/importa/exporta IDs de A, Then Node deniega acceso y jamás envía evidencia de A a un job de B.
6. Medir tiempo, memoria y coste de arranque con corpus y hardware declarados; comparar transporte local con alternativa de servicio solo si hay umbral incumplido.

La aceptación técnica requiere revisión humana de contrato, seguridad y operación. El código Go futuro exige `go test ./...`, fuzzing de validadores/reglas relevantes y cobertura CodeQL/CI añadida en su PR; no se puede declarar Go validado por los checks TS/Java actuales. Fuentes oficiales consultadas: [Prisma/PostgreSQL](https://www.prisma.io/docs/prisma-orm/quickstart/postgresql), [GitHub OAuth](https://docs.github.com/en/apps/oauth-apps/building-oauth-apps/authorizing-oauth-apps), [Node child process](https://nodejs.org/api/child_process.html), [Go fuzzing](https://go.dev/doc/security/fuzz/) y [Railway Dockerfiles](https://docs.railway.com/builds/dockerfiles).
