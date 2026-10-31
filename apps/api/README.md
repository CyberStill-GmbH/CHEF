# CHEF API

Servicio HTTP mínimo del prototipo. Reutiliza el parser Nmap y el caso de uso de importación existentes; no reimplementa reglas en la capa de red.

## Qué está implementado y probado

- `GET /health` sin autenticación.
- `POST /auth/github/start` y `GET /auth/github/callback` con `state` firmado y de un solo propósito.
- `POST /projects` y `GET /projects` con sesión de portador.
- `POST /projects/:id/imports` con cuerpo XML; aplica scope, límites y validación del núcleo.
- `GET /projects/:id/snapshot` devuelve el `Snapshot 1.0.0` validado.
- Aislamiento entre proyectos: una membresía ausente responde `NOT_FOUND`, nunca `FORBIDDEN`, para no revelar existencia.
- Cabeceras de endurecimiento, límite de cuerpo y errores saneados (sin stack, sin rutas, sin SQL).
- Pruebas en `tests/api.test.ts`, `tests/projects.test.ts` y `tests/session-token.test.ts`.

## Cómo se ejecuta

```sh
export CHEF_SESSION_SECRET="<32+ caracteres>"
export GITHUB_CLIENT_ID="<id>"
export GITHUB_CLIENT_SECRET="<secreto>"
export GITHUB_REDIRECT_URI="http://127.0.0.1:8787/auth/github/callback"
npm run engine:build
npm run api
```

Variables opcionales: `PORT` (8787), `CHEF_SCOPE` (`fixtures/scope/lab.json`), `CHEF_GO_BINARY` (binario local confiable; por defecto `artifacts/bin/correlate`, con `.exe` en Windows). El Dockerfile incluye el binario Go. Véase el [transporte Node–Go](../../docs/architecture/go-transport.md) para límites, errores y pruebas. Antes de `npm run check`, compilar el motor con `npm run engine:build`.

Sin `CHEF_SESSION_SECRET` o sin credenciales de GitHub el proceso **no arranca**: no existe modo "sin autenticación".

En producción también exige `DATABASE_URL` y `CHEF_ALLOWED_GITHUB_USER_IDS` (IDs numéricos separados por coma). La allowlist se aplica en el callback y en cada petición autenticada. Para migrar: `npm run db:migrate`. Véanse [operaciones de base de datos](../../docs/architecture/database-operations.md) y [despliegue privado](../../docs/architecture/private-deployment.md).

## Estado real

| Pieza                       | Estado                                                                                                  |
| --------------------------- | ------------------------------------------------------------------------------------------------------- |
| Rutas, sesión y aislamiento | Implementado y probado                                                                                  |
| Persistencia                | Prisma cuando hay `DATABASE_URL`; memoria solo para desarrollo sin base                                 |
| PostgreSQL/Prisma           | Migración, aislamiento, reinicio y backup/restore probados contra PostgreSQL 16 temporal                |
| OAuth GitHub                | Adaptador real con `fetch`, probado con proveedor falso; no probado contra GitHub                       |
| Motor Go                    | Integrado localmente para `open-service/v1`; pruebas con binario real; revisión y despliegue pendientes |
| Despliegue                  | No realizado                                                                                            |

El modo en memoria de desarrollo pierde los datos al reiniciar. La prueba `npm run db:verify` utiliza PostgreSQL real y sesiones sintéticas; no acredita OAuth real ni despliegue remoto.
