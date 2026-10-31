# Registro de pruebas manuales

Actualizado 30-10-2026. Este registro separa lo **ejecutado y observado** de lo que sigue **pendiente**. Una prueba no se marca como superada por figurar en un documento: se anota comando, entorno, resultado esperado, resultado observado y evidencia. Si algo falla, se registra el fallo; no se borra ni se reinterpreta.

Entorno de las pruebas ejecutadas: Windows 11, Node.js v26.12.1 (el proyecto declara `>=24 <25`; el CI fija 24), sin Docker daemon, sin `psql`, sin `go`, sin credenciales reales de GitHub y sin dominio autorizado para OSINT.

## Ejecutadas

| Fecha      | Área              | Acción                                                                      | Esperado                                                       | Observado                                                                                                        | Evidencia                       |
| ---------- | ----------------- | --------------------------------------------------------------------------- | -------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------- | ------------------------------- |
| 30-10-2026 | Calidad           | `npm run check`                                                             | Todo verde                                                     | EXIT=0; 56/56 core, 34/34 web, 95 documentos, 10 módulos, Prettier limpio                                        | salida de consola               |
| 30-10-2026 | Core/CLI          | `npm run demo`                                                              | Snapshot 1.0.0 válido con conteos de limpieza                  | `openKept:3`, `skippedOpenFiltered:1`, `uniqueServices:2`, `isConfirmedVulnerability:false`                      | salida JSON + stderr `cleaning` |
| 30-10-2026 | Benchmark         | `npm run benchmark`                                                         | Dos fuentes, IDs trazables                                     | Nmap p50 ≈ 0,26–0,52 ms; Common Crawl p50 ≈ 0,07–0,13 ms; `traceableEntityRatio:1` en ambas                      | salida JSON (dataset sintético) |
| 30-10-2026 | API               | `npm run smoke`                                                             | Arranque, `/health` 200, `/projects` 401, 404, sin secretos    | Verificado: health 200, 401 `UNAUTHORIZED`, 404, sin fuga en stdout/stderr; sin `CHEF_SESSION_SECRET` sale con 1 | `scripts/api-smoke.mjs`         |
| 30-10-2026 | API (manual)      | `node dist/apps/api/src/main.js` + `curl /health` y `/projects`             | 200 y 401                                                      | 200 `{"status":"ok"}`; 401 `{"error":"UNAUTHORIZED"}`                                                            | consola + curl                  |
| 30-10-2026 | Web               | `npm run web:check`                                                         | Tipos, 34 pruebas y build                                      | Verde; bundle ≈ 385 kB (114 kB gzip)                                                                             | salida de Vite                  |
| 30-10-2026 | Backup (en seco)  | `node scripts/backup-db.mjs` sin `DATABASE_URL` y sin `pg_dump`             | Error claro, sin tocar datos                                   | Exit 2 sin URL; exit 3 con “pg_dump no está instalado”                                                           | consola                         |
| 30-10-2026 | Restore (en seco) | `node scripts/restore-db.mjs` con archivo ausente, sin `--yes`, digest malo | Rechazo con código distinto por causa                          | Exit 2 (uso / confirmación), exit 3 (digest), exit 3 (falta `pg_restore`)                                        | consola                         |
| 30-10-2026 | Adaptador Prisma  | Pruebas con cliente falso (`tests/prisma-projects.test.ts`)                 | Mapeo, conflicto, aislamiento, snapshot reciente, payload roto | Verde en `npm run check`                                                                                         | pruebas automáticas             |

## Pendientes de ejecución real

| Prioridad | Prueba                                                        | Por qué no se ejecutó                                           | Cómo se cierra                                                                            |
| --------- | ------------------------------------------------------------- | --------------------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| P0        | Migración `0001_init` contra PostgreSQL                       | Sin Docker daemon ni `psql`; `prisma` CLI no es dependencia aún | Levantar `docker compose up -d db`, añadir `prisma`, aplicar migración y verificar tablas |
| P0        | Aislamiento con dos proyectos y dos usuarios sobre PostgreSQL | Depende de la migración y del adaptador compuesto               | Componer `PrismaProjectRepository`, repetir el caso A/B del test HTTP y comprobar 404     |
| P0        | OAuth real contra GitHub                                      | Sin `GITHUB_CLIENT_*` ni callback público                       | Crear OAuth app, configurar `GITHUB_REDIRECT_URI`, recorrer start → callback → token      |
| P1        | `docker build` de la imagen de la API                         | Docker daemon apagado                                           | `docker build -t chef-api .` y smoke contra el contenedor                                 |
| P1        | Backup y restore reales                                       | Sin `pg_dump`/`pg_restore` ni base                              | Crear datos, ejecutar backup, restaurar en base limpia y comparar recuentos               |
| P1        | Despliegue privado (Vercel/Railway)                           | Sin infraestructura ni presupuesto confirmado                   | Definir proyecto, secretos, base y URL; repetir el guion del runbook                      |
| P2        | Motor Go                                                      | `go` no instalado y contrato `CorrelationJob` sin aprobar       | Aprobar ADR/contrato, añadir CI Go, regla `open-service/v1` y conformidad TS/Go           |
| P2        | Accesibilidad y navegación real en navegador                  | Solo pruebas DOM; falta revisión manual                         | Recorrido con teclado, lector de pantalla y `aria-live`; corregir deuda del panel         |
| P2        | Common Crawl real                                             | Sin dominio autorizado con XML Nmap correspondiente             | Cerrar el gate de `source-gate-common-crawl.md` y registrar cuota, fecha y resultado      |

## Cómo se registra una prueba nueva

1. Anotar fecha, commit (`git rev-parse --short HEAD`), sistema, versión de Node y si había red.
2. Escribir el comando exacto o los pasos de UI, sin secretos ni datos de clientes.
3. Registrar resultado esperado y observado por separado.
4. Adjuntar evidencia mínima (salida, captura redactada o digest). No adjuntar XML real ni cookies.
5. Si falla: dejarlo como fallo abierto con causa y siguiente paso; no marcarlo como “pendiente” para ocultarlo.
