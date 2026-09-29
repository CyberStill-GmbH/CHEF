# Arquitectura propuesta de CHEF web

Fecha 29-09-2026. **Propuesta para revisión, no arquitectura implementada.** El propietario fijó React/TypeScript, API Node.js/TypeScript con GitHub OAuth y PostgreSQL/Prisma, y motor principal de correlación Go. El parser/caso de uso Nmap TypeScript y la CLI existentes siguen estables. El [ADR 0011](../adr/0011-node-go-boundary.md) define la frontera y los gates de conformidad; la selección de stack no acredita que la web o Go ya funcionen.

## Contexto y límites de confianza

```mermaid
flowchart LR
  Analyst["Pentester autorizado"] -->|HTTPS| Web["React/TypeScript<br/>Vercel propuesto"]
  Web -->|API autenticada| API["CHEF API<br/>Node/TypeScript · Railway propuesto"]
  API -->|OAuth web flow| GitHub["GitHub OAuth"]
  API -->|puertos| Import["Parser y caso de uso TS actuales<br/>Nmap · scope · evidencia"]
  API -->|job JSON versionado| Go["Motor Go propuesto<br/>correlación · deduplicación · explicación"]
  API -->|adaptador Prisma| PG[("PostgreSQL<br/>proyectos · runs · observaciones")]
  API -->|importación local autorizada| Nmap["XML Nmap<br/>no amplía scope"]
  API -.->|conector candidato sujeto a permiso| Passive["Fuente OSINT pasiva"]
  Future["Futuro worker activo<br/>scope aprobado, cuotas, egress"] -.->|resultado importado| API
  Burp["Futura BApp Java/Montoya"] -.->|contrato versionado| API
```

Fronteras: navegador no recibe secretos de proveedor ni acceso directo a PostgreSQL; API valida identidad, membresía, tamaño, formato y scope; adaptadores tratan XML/OSINT como no confiables. Go recibe observaciones canónicas de **un solo proyecto**, sin tokens, red ni acceso directo a DB; devuelve assertions auditables. El dominio TS existente no conoce HTTP, base, React ni Burp. Fuentes externas, Go y worker activo son **propuestos**, no disponibles hoy. El importador Nmap y core TS sí existen; API/web/DB no.

## Contenedores y dependencias

```mermaid
flowchart TB
  UI["apps/web · presentación React<br/>grafo/lista · panel de evidencia"] -->|DTO versionado| HTTP["apps/api · composición<br/>OAuth/session · autorización · límites"]
  HTTP --> UC["packages/application<br/>ImportEvidence actual · puertos futuros"]
  UC --> Domain["packages/domain<br/>identidad Nmap · evidencia actuales"]
  HTTP --> EnginePort["CorrelationEngine port<br/>job/resultado versionados"]
  EnginePort --> Go["motor Go propuesto<br/>reglas multifuente puras"]
  HTTP --> Adapters["packages/adapters<br/>Nmap XML · Prisma · OSINT futuro"]
  Adapters --> UC
  Adapters --> DB[("PostgreSQL")]
  CLI["apps/cli · importación/regresión"] --> UC
  CLI --> Adapters
```

La dirección lógica de dependencias es hacia dominio/application. El adaptador Prisma implementa puertos definidos por casos de uso; otro puerto abstrae Go. Node conserva parsing/scope y es la única puerta a datos/autorización. Go será la **única implementación autoritativa de reglas multifuente nuevas**; el core TS vigente permanece como regresión/semántica de Nmap durante la migración, no como segundo motor web. HTTP expone DTO de lectura; `Snapshot 1.0.0` sigue como formato de exportación/intercambio, no modelo ORM. Añadir observaciones pasivas al wire requiere nueva versión/esquema/ejemplo/prueba/compatibilidad. La frontera permite sustituir transporte sin contaminar reglas con Prisma o HTTP.

## Flujo de la primera vertical

```mermaid
sequenceDiagram
  actor P as Pentester
  participant W as React
  participant A as API
  participant C as Parser/core TS vigente
  participant G as Motor Go propuesto
  participant D as PostgreSQL
  P->>W: Selecciona proyecto e importa XML autorizado
  W->>A: Solicitud autenticada + proyecto + policy
  A->>A: Verifica membresía, tamaño, tipo, scope
  A->>C: ImportEvidence(XML, policy)
  C-->>A: Snapshot versionado + evidencias + errores
  A->>D: Transacción idempotente por proyecto/run
  A->>G: CorrelationJob versionado con observaciones del proyecto
  G-->>A: Assertions + regla/versión/evidencias o error
  A->>D: Publicación atómica del resultado validado
  A-->>W: Resumen de importación + conteos de exclusión
  P->>W: Abre servicio y relación
  W->>A: Consulta acotada al proyecto
  A->>D: Lee observación, fuente, regla y fecha
  A-->>W: Relación observada/candidata + explicación
  W-->>P: Grafo/lista + panel de evidencia
```

Cuando exista una fuente pasiva autorizada, su adaptador crea observaciones independientes con locator, tiempo, términos de origen y estado de actualización. Node construye un job acotado al proyecto; Go compara claves tipadas y contexto temporal, devuelve candidatos/contradicciones y nunca recibe credenciales. La primera hipótesis de transporte es binario Go local por `stdin/stdout` sin shell, timeout y límites; el spike debe validarla antes de comprometer Railway. No invoca Nmap ni dirige tráfico al objetivo. Fallo de Go no publica relación parcial. Cada ejecución guarda regla/parser/versiones para reproducir el enlace.

## Acceso a datos y multiusuario

Modelo lógico propuesto: `User` ↔ `Membership` ↔ `Project`; dentro de proyecto `ScopePolicy`, `Source`, `ImportRun`, `Evidence`, `Observation`, `Entity`, `RelationAssertion`, `ReviewDecision`, `Export`. `projectId` y constraints de integridad están en toda entidad de proyecto; consultas/actualizaciones se autorizan por membresía y acción. RLS de [PostgreSQL](https://www.postgresql.org/docs/current/ddl-rowsecurity.html) puede ser defensa adicional tras prueba de contexto por conexión. El dueño de tabla no está sujeto por defecto a políticas RLS: configurar roles de ejecución con cuidado. La API jamás confía en un `projectId` enviado por cliente sin comprobar pertenencia. Exportaciones y objetos temporales también se acotan. GitHub OAuth solo prueba identidad; no concede permisos de proyecto.

Persistir metadata de evidencia y política de retención; decidir si el XML original se guarda cifrado o no se retiene. Hashes de archivos permiten integridad local, no autenticidad ni anonimización. Definir backup/restore, borrado, migraciones reversibles y logs sin payload sensible antes de publicar. [Prisma/PostgreSQL](https://www.prisma.io/docs/prisma-orm/quickstart/postgresql) es la interfaz elegida por el propietario; fijar versión estable en PR de implementación.

## Contrato de explicación y visualización

Un `RelationView` necesita entidad izquierda/derecha tipada, estado `observed|inferred|candidate|conflicted`, `ruleId` y versión, evidencias a favor/en contra, fechas por fuente, frescura y motivo de exclusión de alternativas. La API decide identidad y fusión; React presenta y filtra. Un mapa descargable tiene **dos productos distintos**: exportación de datos versionada (JSON/schema/manifest) para reproducir, e imagen SVG/PNG de la vista filtrada para informe; se deben probar contra inyección y pérdida de evidencia. HTML interactivo autónomo queda posterior y requiere revisión de seguridad. El grafo siempre tiene lista accesible equivalente y el panel no interpreta HTML importado.

## Calidad, operación y gates

1. Core/CLI existente: tests de regresión, IDs, control negativo, scope y `npm run check`.
2. API/DB: dos proyectos y dos usuarios, pruebas IDOR, transacción atómica, idempotencia, límites de importación, autorización de export, migración/rollback.
3. Conector pasivo: permiso, cuota, provenance, fixture, error/redacción/frescura, cero resultados inventados.
4. Correlación Go: schema/ejemplos/validadores TS y Go, `go test ./...`, fixtures de conformidad y negativos, corpus etiquetado, fallos/timeout/salida inválida, reproducibilidad por versión de regla y benchmark declarado; no afirmar vulnerabilidad confirmada.
5. UI: teclado, lista equivalente, contraste, estados vacío/error, test end-to-end del recorrido y medición con usuarios.
6. Despliegue privado: secretos, OAuth callback, aislamiento, backups/restore, observabilidad sin datos sensibles y runbook. Acceso a [Vercel](https://vercel.com/docs/frameworks/frontend/vite)/[Railway](https://docs.railway.com/guides/saas-backend) no implica despliegue probado.

Escalado posterior: medir el coste de ejecutar Go local antes de separar un servicio, añadir colas, SSE o cache; actualización continua exige proveedor, licencia, latencia y operación definidos. Un worker activo exige alcance aprobado y controles de egress. Burp se integra después con contrato y fixtures de conformidad Java/TS/Go; su validación manual/criterios siguen separados.
