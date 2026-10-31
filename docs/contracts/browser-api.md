# Sesión de navegador y proyectos (prototipo v0)

Incremento aditivo del prototipo HTTP existente. No acepta el diseño `/v1` ni modifica `Snapshot 1.0.0`. Los clientes bearer y el callback JSON del flujo CLI permanecen compatibles.

## Sesión

| Ruta de la API                    | Comportamiento                                                                                                                                                                                                                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `POST /auth/github/browser/start` | Requiere `Origin` igual a `CHEF_WEB_ORIGIN` y `X-Chef-Request: 1`; devuelve `authorizeUrl` y fija cookie temporal HttpOnly de estado.                                                                                                                                    |
| `GET /auth/github/callback`       | Con cookie de navegador, compara el estado, valida su firma/propósito y permite solo cuentas habilitadas. Emite cookie HttpOnly y responde 303 a `CHEF_WEB_ORIGIN`. Fallos redirigen a `/?auth=failed`. Sin cookie de navegador conserva el callback JSON del flujo CLI. |
| `GET /auth/session`               | Devuelve `{ "userId": "github:1" }` si la sesión es válida; 401 si falta/caduca y 403 si la cuenta ya no está habilitada. No devuelve el token.                                                                                                                          |
| `POST /auth/logout`               | Valida origen/cabecera y elimina las cookies. No revoca copias de tokens stateless existentes: siguen sujetos a vencimiento y allowlist.                                                                                                                                 |

Esquemas y ejemplos: [sesión](browser-session.schema.json), [válido](examples/browser-session.json), [inválido](examples/browser-session.invalid.json); [autorización](browser-authorize.schema.json), [válido](examples/browser-authorize.json), [inválido](examples/browser-authorize.invalid.json). `npm run check:docs` comprueba ambos casos. Las pruebas HTTP validan cookies, estados distintos, origen, redirección, logout y rechazo de cuentas.

Cada inicio usa un nonce aleatorio firmado con propósito específico, ligado a una cookie de ese navegador. Cookies: `Path=/`, `HttpOnly`, `SameSite=Lax`; con origen HTTPS añaden `Secure` y prefijo `__Host-`. El vencimiento del token manda aunque el navegador conserve la cookie. Las escrituras con cookie requieren origen exacto y cabecera de petición; no se habilita CORS abierto. El token no se guarda en `localStorage`, `sessionStorage`, URL ni estado React.

## Proyectos y resultados

La web llama a `/api/...` en su propio origen. El proxy elimina `/api` antes de llegar al backend:

- `GET /projects`: proyectos autorizados de la cuenta actual.
- `POST /projects` con `{ "name": "Laboratorio" }`: crea proyecto y membresía del propietario; no permite elegir otro propietario.
- `POST /projects/{id}/imports`: XML crudo con `Content-Type: application/xml`. La UI acepta hasta 2 MiB; siguen aplicándose los límites y scope de la API. Tras respuesta válida recarga el snapshot persistido.
- `GET /projects/{id}/snapshot`: snapshot validado por esquema y referencias antes de representarlo. El 404 se presenta como snapshot no disponible, sin distinguir un proyecto inaccesible de uno sin importaciones.

Al cambiar de proyecto o cerrar sesión se abortan las solicitudes anteriores. Las respuestas tardías no se aplican a otro proyecto. Un fallo de importación conserva la vista previa y presenta un error saneado. La autorización por membresía sigue ejecutándose en el servidor en cada operación.

## Configuración y compatibilidad

`CHEF_WEB_ORIGIN=http://127.0.0.1:5173` en la API. `npm run web:dev` escucha allí y falla si el puerto está ocupado para no cambiar silenciosamente el origen. `CHEF_API_PROXY_TARGET` permite elegir otra API local; predeterminado `http://127.0.0.1:8787`. Usar el mismo hostname (`127.0.0.1`) en web y callback local, aunque los puertos difieran. El callback local existente puede seguir en `http://127.0.0.1:8787/auth/github/callback`.

Para Vercel/Railway, configurar proxy HTTPS `/api` hacia la API y registrar el callback `https://<web>/api/auth/github/callback`. Así el estado y la sesión pertenecen al origen web; no depender de cookies de terceros entre dominios de hosting. No hay despliegue remoto en este incremento.

La demo offline exige una acción explícita y está etiquetada como sintética. La pantalla inicial ya no presenta el fixture como proyectos guardados. La aceptación formal y la validación manual de GitHub real siguen pendientes de revisión del equipo.
