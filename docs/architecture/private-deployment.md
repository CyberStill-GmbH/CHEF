# Preparación del despliegue privado

Destino elegido: Railway para API/PostgreSQL y Vercel para la web. Archivos preparados y contenedor probado localmente; no implica que exista un despliegue remoto. Antes de publicar datos reales se requiere la revisión del equipo y la validación del entorno.

## Herramientas y acceso

Railway CLI y Vercel CLI son las herramientas del operador. Comprobar `railway --version` y `vercel --version`, e iniciar sesión con `railway login` y `vercel login`. En Windows, si los comandos no están en PATH, consultar `npm prefix --global` y ejecutar sus archivos `.cmd` desde esa carpeta. Nunca pegar tokens ni secretos en el chat o en Git.

## Railway: API y base de datos

1. Crear un proyecto/entorno de prueba y añadir PostgreSQL. Usar una base nueva para la primera validación.
2. Conectar el repositorio y seleccionar una rama revisada que incluya API, motor Go y estas operaciones. La raíz de build es la raíz del repositorio; `railway.json` elige Dockerfile, ejecuta la migración como pre-deploy y comprueba `/health`.
3. Configurar las variables de la API:

| Variable                                   | Valor                                                                                          |
| ------------------------------------------ | ---------------------------------------------------------------------------------------------- |
| `DATABASE_URL`                             | Referencia a la URL de PostgreSQL del servicio, preferiblemente por la red privada de Railway. |
| `CHEF_SESSION_SECRET`                      | Secreto aleatorio de al menos 32 caracteres, exclusivo del entorno.                            |
| `GITHUB_CLIENT_ID`, `GITHUB_CLIENT_SECRET` | Credenciales de la OAuth App elegida para CHEF.                                                |
| `GITHUB_REDIRECT_URI`                      | URL HTTPS de la API seguida de `/auth/github/callback`; debe coincidir con la OAuth App.       |
| `CHEF_ALLOWED_GITHUB_USER_IDS`             | IDs numéricos de las cuentas autorizadas, separados por comas.                                 |
| `HOST`                                     | `0.0.0.0`. Railway proporciona `PORT`.                                                         |
| `NODE_ENV`                                 | `production` (ya establecido por la imagen).                                                   |
| `CHEF_SCOPE`                               | Política explícita autorizada; el fixture predeterminado solo admite laboratorio sintético.    |

Obtener el ID estable de una cuenta con `gh api users/USUARIO --jq .id`. La allowlist se comprueba al emitir sesión y en cada petición autenticada. Retirar un ID también bloquea sus tokens existentes. En producción, el arranque exige `DATABASE_URL` y una allowlist no vacía; así no se publica una API que persiste en memoria ni un registro abierto a cualquier cuenta GitHub.

4. Desplegar primero el entorno de prueba. La imagen contiene Node, Go y el cliente Prisma 6.19.3. No instala dependencias en el arranque. Si la migración falla, no iniciar la nueva versión de la API.
5. Probar `/health`, login GitHub real de una cuenta habilitada, rechazo de una cuenta no habilitada, importación con scope, aislamiento entre dos usuarios y persistencia tras reinicio. Verificar backup/restore en una base de recuperación siguiendo [operaciones de PostgreSQL](database-operations.md).

OAuth autentica al usuario y la allowlist restringe la instancia; las membresías siguen autorizando cada proyecto. El flujo de navegador utiliza cookies HttpOnly y redirección; el callback JSON se conserva para CLI. Configurar CHEF_WEB_ORIGIN y el proxy /api conforme al contrato de navegador.

## Vercel: web

`vercel.json` usa la raíz del repositorio, `npm ci`, `npm run web:build` y `apps/web/dist` como salida. Configurar Node 24. `.vercelignore` excluye secretos, backups y resultados locales.

Crear primero una preview bajo la cuenta/equipo del proyecto. Activar la protección de despliegues de Vercel y verificar que personas ajenas no puedan abrirla; no asumir privacidad solo por ser preview. No habilitar producción pública por defecto ni introducir secretos de servidor en variables `VITE_*`.

**Estado del incremento web:** la pantalla inicial consulta la sesión, lista/crea proyectos e importa XML contra la API; la demo sintética es explícita. La integración se valida localmente con PostgreSQL y Go. Antes de alojarla, configurar un proxy HTTPS `/api` desde Vercel hacia Railway y registrar el callback `https://<web>/api/auth/github/callback`; `CHEF_WEB_ORIGIN` debe ser ese origen HTTPS. La configuración estática de Vercel por sí sola no incluye un destino de API: añadirlo cuando se conozca el dominio, sin abrir CORS ni depender de cookies de terceros. El despliegue remoto y OAuth real requieren su validación.

## Reversión

Conservar la versión anterior de la aplicación y un backup verificado antes de migraciones sobre datos existentes. Las migraciones futuras deben ser compatibles con el rollback del código o tener un procedimiento explícito de recuperación. Un rollback de Railway no revierte automáticamente la base. No ejecutar `docker compose down -v` sobre los datos del equipo.

Fuentes: [configuración de Railway](https://docs.railway.com/config-as-code/reference), [Dockerfiles en Railway](https://docs.railway.com/builds/dockerfiles), [configuración de Vercel](https://vercel.com/docs/project-configuration), [protección de despliegues](https://vercel.com/docs/deployment-protection).

La API requiere `CHEF_WEB_ORIGIN` en producción. Detalles de cookies, compatibilidad y rutas: [contrato de navegador](../contracts/browser-api.md).
