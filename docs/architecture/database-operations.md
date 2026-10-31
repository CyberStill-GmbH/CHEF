# PostgreSQL, backup y restauración desde el host

Implementado y validado localmente con PostgreSQL 16 en Docker y Node 24 sobre Windows. Los scripts se ejecutan desde la raíz del repositorio, sin instalar un servidor PostgreSQL adicional en el host.

## Aplicar migraciones

La dependencia Prisma está fijada a `6.19.3` y se incluye en producción para que la imagen pueda ejecutar su CLI sin descargar herramientas durante el despliegue:

```powershell
$env:DATABASE_URL = 'postgresql://chef:<contraseña>@127.0.0.1:5432/chef?schema=public'
npm run db:migrate
```

En Compose: arrancar `db`, ejecutar `docker compose run --rm api npm run db:migrate` y después arrancar `api`. No ejecutar `migrate dev` ni `db push` contra la base de despliegue. En Railway, `railway.json` configura `npm run db:migrate` como pre-deploy.

## Backup

```powershell
npm run db:backup
# O elegir una carpeta local:
npm run db:backup -- 'artifacts\backup'
```

Se generan `chef-<fecha>-<identificador>.dump` y su `.sha256`. El dump usa formato custom de PostgreSQL. Durante la escritura se utiliza `.partial`; un fallo del cliente elimina ese archivo. No sobrescribe un backup anterior. `artifacts/` está ignorado por Git y excluido de los uploads de despliegue. Los dumps contienen datos del proyecto: el formato custom comprime, pero no cifra. Guardarlos en almacenamiento privado y, cuando corresponda, cifrado; la copia local no sustituye una copia fuera del equipo.

La URL Prisma puede contener `schema=public`; el script elimina ese parámetro antes de usar libpq. El backup abarca la base completa, no solo ese esquema. Admite también `sslmode` y `connect_timeout`; otras opciones se rechazan para evitar cambios silenciosos de conexión.

## Elegir cliente

| Variable                    | Función                                                                                                 |
| --------------------------- | ------------------------------------------------------------------------------------------------------- |
| `CHEF_DB_TOOLS_MODE=auto`   | Usa `pg_dump`/`pg_restore` nativos si están en PATH; si faltan, usa Docker. Predeterminado.             |
| `CHEF_DB_TOOLS_MODE=docker` | Fuerza un contenedor efímero del cliente PostgreSQL.                                                    |
| `CHEF_DB_TOOLS_MODE=native` | Exige cliente PostgreSQL instalado en el host.                                                          |
| `CHEF_POSTGRES_IMAGE`       | Imagen del cliente, predeterminada `postgres:16-alpine`; elegir una versión compatible con el servidor. |
| `CHEF_DB_TOOLS_NETWORK`     | Red de Docker opcional, para acceder a la base por nombre de servicio/contenedor.                       |

En modo Docker, `localhost`, `127.0.0.1` y `::1` se traducen a `host.docker.internal`. En Windows se probó el acceso al puerto local publicado. Para una base dentro de una red Docker se puede usar su nombre y `CHEF_DB_TOOLS_NETWORK`; ese recorrido también está probado. No usar una URL privada de Railway desde el host sin conectividad a su red: elegir la conexión accesible al operador, con TLS según la configuración del servicio.

El script transmite el dump como bytes, sin redirecciones de texto de PowerShell ni montajes de la carpeta de backups. Las contraseñas no forman parte de los argumentos de proceso: utiliza un archivo temporal de credenciales. En Docker se copia a un archivo Linux privado para respetar los permisos de libpq, y el contenedor se elimina al terminar. Los mensajes del cliente no se imprimen porque pueden contener detalles de conexión. Un fallo indica herramienta/modo; revisar conectividad, credenciales, permisos y compatibilidad de versión.

## Restore

Crear primero la base de recuperación y apuntar `DATABASE_URL` a ella. Restaurar reemplaza los objetos presentes en el archivo; no crea la base, no elimina todos los objetos ajenos al dump y no restaura roles globales. Detener las escrituras de la aplicación cuando se restaure sobre una base en uso.

```powershell
$env:DATABASE_URL = 'postgresql://chef:<contraseña>@127.0.0.1:5432/chef_recuperada?schema=public'
npm run db:restore -- 'artifacts\backup\chef-<fecha>-<identificador>.dump' --yes
```

Antes de conectar verifica el SHA-256 y el nombre del sidecar. Sin `--yes`, sin sidecar válido o con el archivo alterado, rechaza la operación. Usa `--clean --if-exists --single-transaction --exit-on-error`: si falla una operación de restauración, revierte la transacción. El digest comprueba integridad local, no la identidad de quien produjo el backup; restaurar solo archivos de una fuente confiable.

Después, comprobar tablas, conteos, proyectos, snapshots y acceso de dos usuarios; conservar el dump original hasta validar la recuperación.

## Verificación reproducible

```sh
docker build -t chef-db-operations:local .
npm run db:verify
```

La prueba crea contenedores, una red y dos bases nuevas con credenciales sintéticas; no utiliza `DATABASE_URL` del operador. Comprueba migración idempotente, tablas, dos usuarios/proyectos, importación con Go, persistencia al reiniciar y restauración en otra base. Compara los datos y el snapshot leído por HTTP antes/después. Rechaza un dump alterado, exige confirmación e induce un fallo DDL para demostrar rollback sin perder datos del destino temporal. Elimina únicamente los recursos creados por la prueba y conserva evidencia sintética en `artifacts/db-verification/`.

Esta prueba usa sesiones sintéticas; no acredita OAuth real, acceso a Railway ni recuperación de una base de producción. La selección automática hacia Docker y la restauración desde un puerto local se verificaron adicionalmente en Windows; el modo nativo no se ejecutó en ese host.

Fuentes: [pg_dump](https://www.postgresql.org/docs/16/app-pgdump.html), [pg_restore](https://www.postgresql.org/docs/16/app-pgrestore.html), [variables de libpq](https://www.postgresql.org/docs/16/libpq-envars.html).
