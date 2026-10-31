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
npm run engine:build # compila el motor Go local
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

La API usa Prisma/PostgreSQL cuando hay `DATABASE_URL`. En producción exige esa conexión y una allowlist de IDs GitHub; el modo en memoria queda para desarrollo. Véase [despliegue privado](../architecture/private-deployment.md).

### Web local

```sh
npm run web:dev     # Vite sirve la UI con el snapshot fixture, sin sesiones
npm run web:check   # tipos + 34 pruebas + build
```

La web todavía consume el fixture canónico; no está conectada a la API y no tiene grafo ni filtros.

## Base de datos, contenedor y recuperación

La migración, aislamiento entre usuarios, persistencia tras reiniciar y backup/restore se validaron contra PostgreSQL 16 temporal. Para repetir sin tocar datos existentes:

```sh
docker build -t chef-db-operations:local .
npm run db:verify
```

Para una base elegida por el operador: configurar `DATABASE_URL` y ejecutar `npm run db:migrate`. En Compose, levantar `db`, ejecutar `docker compose run --rm api npm run db:migrate` y arrancar `api` con la configuración completa de `.env`. La imagen contiene Node, Go y Prisma.

`npm run db:backup` genera dump y SHA-256. `npm run db:restore -- <archivo.dump> --yes` verifica integridad y restaura en una transacción. Si faltan clientes PostgreSQL en Windows, los scripts usan Docker automáticamente. No utilizar comodines en lugar del nombre exacto del archivo. Las instrucciones y variables están en [operaciones de base de datos](../architecture/database-operations.md).

El despliegue remoto sigue pendiente de cuentas/configuración y validación en Railway/Vercel. La web todavía usa un fixture; no tiene sesión ni conexión con la API.

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
| El snapshot no aparece tras reiniciar | Persistencia en memoria                              | Comprobar DATABASE_URL y la base/entorno consultado   |

## Registro y evidencia

Cada ensayo debe anotar commit, entorno, comando, resultado esperado/observado y evidencia mínima en el [registro de pruebas manuales](../testing/manual-test-log.md). No adjuntar XML real, tokens, cookies, capturas con dominios de clientes ni exportaciones con datos sensibles.
