# Runbook de demostración CHEF

Actualizado 30-10-2026. **Objetivo del prototipo, con lo construido y lo pendiente marcados:** prototipo web privado para el equipo en octubre. Recorrido: iniciar con GitHub → crear proyecto → importar Nmap autorizado → consultar fuente pasiva autorizada → revisar servicio y relación con dos evidencias/fechas → exportar. No lanzar escaneos desde CHEF. Antes de cada ensayo anotar commit, entorno, fuente, permiso, versión de reglas y qué pasos están implementados. Un fixture sintético se etiqueta como sintético; no se presenta como respuesta OSINT viva.

## Preparación y gates

1. Confirmar que `npm ci`, `npm run check`, build Java si aplica y CI están verdes. Preparar proyecto de prueba vacío y cuentas miembro/no miembro. Probar migración, backup/restore y callback OAuth **si** el despliegue privado existe.
2. Usar XML Nmap/alcance autorizados y la fuente pasiva aprobada con su política de retención; no utilizar datos de clientes en capturas o repo. El fixture [Nmap](../../fixtures/nmap/lab.xml) usa loopback/`.invalid`; sirve para fallback, no para una consulta OSINT real.
3. Preparar respuestas correctas: servicio duplicado con dos observaciones, hostname compartido que **no** fusiona hosts, fuente pasiva con fecha y estado candidato, archivo hostile que falla sin salida parcial. Un evaluador debe poder seguir Observation→Evidence→digest/locator y regla.
4. Ensayar en la URL privada desde navegador distinto y también el caso denegado de otro proyecto. Si seguridad o despliegue no están probados, **no** etiquetar el resultado como multiusuario/desplegado.

## Guion objetivo de cinco minutos

| Tiempo      | Acción que debe verse                  | Evidencia/claim permitido                                                        |
| ----------- | -------------------------------------- | -------------------------------------------------------------------------------- |
| 00:00–00:35 | Problema y proyecto autorizado         | Fuentes/fechas, objetivo de ahorrar revisión manual                              |
| 00:35–01:20 | Login/proyecto/importación Nmap        | Scope y resultado real, conteos de duplicados/exclusiones                        |
| 01:20–02:10 | Fuente pasiva elegida                  | Proveedor, permiso, fecha y error/cuota; sintética si aplica                     |
| 02:10–03:20 | Mapa/lista y servicio enlazado         | Dos cadenas de evidencia y regla; candidato ≠ identidad                          |
| 03:20–04:05 | Control negativo y estado ambiguo Nmap | Hostname compartido no fusiona; `open\|filtered` no es `open`                    |
| 04:05–05:00 | Exportación y contraste manual         | JSON/imagen solo si implementados; tiempo/pasos medidos, límites y próximo corte |

César explica contrato/regla/scope; Diego guía interacción y accesibilidad; Jhojan explica fixture, limpieza, procedencia y fuente. Los tres ensayan y se corrigen mutuamente. Grabar duración/errores/recuperación de al menos dos ensayos antes del 30-10; tres si la disponibilidad permite. No inventar resultados del experimento con pentesters.

## Qué se puede probar hoy

Requisitos: Node.js 24 (`fnm use` / `nvm use`; el repo incluye `.nvmrc`), npm y el repositorio copiado. Los fixtures son sintéticos: loopback y `.invalid`, sin Nmap, Burp, base de datos ni Internet. **No se necesita ninguna credencial real para la regresión.**

```sh
npm ci
npm run check        # lint + tipos + 56 core + 34 web + smoke API + docs + arquitectura + formato
npm run demo         # snapshot 1.0.0 con conteos de limpieza
npm run smoke        # arranca la API compilada y verifica /health, 401 y fallo rápido
```

Si `npm run check` termina en 0, la regresión está verde. Cualquier paso que falle debe registrarse en el [registro de pruebas manuales](../testing/manual-test-log.md), no ocultarse.

### API local

```sh
cp .env.example .env    # completar secretos y credenciales de una OAuth app de prueba
npm run api             # construye y escucha en 127.0.0.1:8787
```

| Comprobación                       | Respuesta esperada                                            |
| ---------------------------------- | ------------------------------------------------------------- |
| `GET /health`                      | `200 {"status":"ok"}`                                         |
| `GET /projects` sin token          | `401 {"error":"UNAUTHORIZED"}`                                |
| `GET /nope`                        | `404 {"error":"NOT_FOUND"}`                                   |
| Arrancar sin `CHEF_SESSION_SECRET` | el proceso termina con código 1 y nombra la variable faltante |

La API actual **persiste en memoria**: al reiniciar se pierden proyectos y snapshots. Es deliberado y está declarado en [apps/api/README.md](../../apps/api/README.md). No presentar esto como multiusuario persistente.

### Web local

```sh
npm run web:dev     # Vite sirve la UI con el snapshot fixture, sin sesiones
npm run web:check   # tipos + 34 pruebas + build
```

La web todavía consume el fixture canónico; no está conectada a la API y no tiene grafo ni filtros.

## Base de datos (pendiente de ejecución)

El esquema y la migración existen en `apps/api/prisma/`, y el adaptador `packages/adapters/src/prisma-projects.ts` está probado con un cliente falso. **Nada de esto se ejecutó contra PostgreSQL.** Para cerrarlo:

```sh
docker compose up -d db          # requiere Docker daemon activo
npm install --save-dev prisma    # el CLI aún no es dependencia del repo
export DATABASE_URL="postgresql://chef:chef-dev-only@127.0.0.1:5432/chef?schema=public"
npx prisma migrate deploy --schema apps/api/prisma/schema.prisma
```

Después hay que **componer** `PrismaProjectRepository` y `PrismaSnapshotStore` en `apps/api/src/main.ts` (hoy usa los adaptadores en memoria) y repetir la prueba de aislamiento con dos usuarios y dos proyectos. Hasta entonces, la API no toca la base.

## Empaquetado

```sh
docker build -t chef-api .       # no ejecutado: daemon apagado en el entorno de entrega
docker compose up api            # imagen + Postgres; requiere .env completo
```

La imagen es multi-stage, corre como usuario `node` y tiene `HEALTHCHECK` contra `/health`. Cuando exista el motor Go se añade su etapa de build y el binario estático.

## Backup y restore

```sh
DATABASE_URL=... npm run db:backup -- artifacts/backup
DATABASE_URL=... npm run db:restore -- artifacts/backup/chef-*.dump --yes
```

Los scripts se verificaron **en seco** (sin URL, sin herramienta, sin confirmación y con digest alterado: cada caso falla con código distinto). El backup real requiere `pg_dump`, el restore requiere `pg_restore` y ambos exigen `DATABASE_URL`. El restore verifica el SHA-256 del sidecar antes de tocar la base y exige `--yes`.

## Rollback

1. Código: `git revert <commit>` o volver a la etiqueta anterior; nunca reescribir `main`.
2. Contenedores: `docker compose down` (sin `-v` conserva el volumen de PostgreSQL).
3. Base: restaurar el último `.dump` verificado con el comando anterior.
4. Si el fallo es de configuración, rotar secretos antes de reintentar.

## Problemas frecuentes

| Síntoma                               | Causa probable                                       | Acción                                                |
| ------------------------------------- | ---------------------------------------------------- | ----------------------------------------------------- |
| `EADDRINUSE` en 8787                  | Otro proceso ocupa el puerto                         | `PORT=8788 npm run api` o detener el proceso anterior |
| `EBADENGINE` al instalar              | Node local fuera de `>=24 <25`                       | `fnm use` / `nvm use` con `.nvmrc`                    |
| La API no arranca                     | Falta `CHEF_SESSION_SECRET` o credenciales de GitHub | Completar `.env`; el fallo es intencional             |
| `npm run smoke` no encuentra `dist`   | Se ejecutó sin build                                 | `npm test` o `npm run build` antes                    |
| `pg_dump`/`pg_restore` no encontrado  | Cliente PostgreSQL ausente                           | Instalarlo o usar `docker compose exec db ...`        |
| El snapshot no aparece tras reiniciar | Persistencia en memoria                              | Esperado; falta componer el adaptador Prisma          |

## Registro y evidencia

Cada ensayo debe anotar commit, entorno, comando, resultado esperado/observado y evidencia mínima en el [registro de pruebas manuales](../testing/manual-test-log.md). No adjuntar XML real, tokens, cookies, capturas con dominios de clientes ni exportaciones con datos sensibles.
