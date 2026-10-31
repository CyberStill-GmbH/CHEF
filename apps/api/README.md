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
npm run api
```

Variables opcionales: `PORT` (8787), `CHEF_SCOPE` (`fixtures/scope/lab.json`).

Sin `CHEF_SESSION_SECRET` o sin credenciales de GitHub el proceso **no arranca**: no existe modo "sin autenticación".

## Estado real

| Pieza                       | Estado                                                                            |
| --------------------------- | --------------------------------------------------------------------------------- |
| Rutas, sesión y aislamiento | Implementado y probado                                                            |
| Persistencia                | Adaptador en memoria (`InMemoryProjectRepository`)                                |
| PostgreSQL/Prisma           | Esquema y migración escritos en `prisma/`, **no ejecutados**                      |
| OAuth GitHub                | Adaptador real con `fetch`, probado con proveedor falso; no probado contra GitHub |
| Motor Go                    | No existe; `go` no está instalado en el entorno de entrega                        |
| Despliegue                  | No realizado                                                                      |

El adaptador en memoria pierde los datos al reiniciar. Es deliberado: preferimos declararlo antes que presentar una persistencia que nadie ejecutó.
