# Runbook de demostración CHEF

Actualizado 31-10-2026. El NSIS final se instaló en Windows. Se verificaron el WebView React, el snapshot local, `chef help`, `chef url` sin tráfico de red y la descarga del informe HTML. La importación manual de archivos, selección de activos y controles negativos aún están pendientes. La demo no requiere cuentas ni servicios remotos.

## Preparación y estado

1. Compilar y validar la web con `npm run web:check`; ejecutar `npm run check:docs`, `npm run format:check` y `npm run check:architecture`. La suite Tauri/Rust y CI deben estar verdes antes de la presentación.
2. Artefactos finales: `CHEF_0.1.0_x64-setup.exe` (30.02 MiB) y `CHEF_0.1.0_x64_en-US.msi` (48.84 MiB). El NSIS se instaló silenciosamente en `%LOCALAPPDATA%\\Programs\\CHEF` con código 0. El proceso `chef-desktop` (PID 13468) mantuvo la ventana `CHEF` con `Responding=True` tras 15 segundos. El build serial final terminó con código 0; no asociar el instalador a un commit hasta que los cambios se confirmen.
3. Tener a mano el fixture sintético [urlscan positivo](../../fixtures/urlscan-response/positive.json) y usar `example.com` como dominio consultado. El [fixture negativo](../../fixtures/urlscan-response/negative.json) sirve para comprobar que resultados fuera de la raíz no aparecen como activos.
4. Mantener la red desconectada durante el recorrido si el entorno lo permite. La demo solo importa archivos locales; no consulta urlscan, DNS, Censys ni objetivos y no habilita descubrimiento activo.
5. Registrar ensayo, duración real, errores y recuperación. No anotar ensayos que no se hayan realizado ni métricas de usuarios inventadas.

## Prueba manual de instalación y arranque

**Estado de esta revisión: instalación y respuesta básica de ventana aprobadas; renderizado e interacción pendientes.** La comprobación visual de la herramienta de escritorio agotó sus reintentos de enumeración de ventanas. El título y la propiedad `Responding` no prueban que React haya renderizado correctamente ni que los comandos IPC/SurrealDB funcionen.

| Criterio | Given | When | Then |
| --- | --- | --- | --- |
| Instalación | Está disponible el NSIS `CHEF_0.1.0_x64-setup.exe` de esta revisión. | Se instala en el perfil local de Windows. | **Aprobado:** salida 0; ejecutable instalado bajo `%LOCALAPPDATA%\Programs\CHEF`. |
| Arranque | CHEF está instalado. | Se inicia `chef-desktop.exe`. | **Aprobado:** el WebView mostró la interfaz React, el snapshot local con 3 activos y la terminal CHEF. La ventana `CHEF` siguió respondiendo. |
| Terminal de comandos | CHEF muestra el snapshot local. | Se ejecuta `chef help`. | **Aprobado:** la terminal muestra los comandos CHEF disponibles; no ejecuta una shell del sistema. |
| Clasificación de URL | CHEF está abierto; se usa `https://example.com/private?token=dummy#test`. | Se ejecuta `chef url <URL>`. | **Aprobado:** Rust clasifica la URL, se omiten query y fragmento, y CHEF informa que no hizo consultas de red. |
| Informe local | Hay un snapshot cargado. | Se ejecuta `chef report`. | **Aprobado:** se descarga un informe HTML local con nombre saneado; el archivo se verificó en Descargas. |
| Exploración offline | CHEF muestra el fixture sintético inicial. | Se abre Evidencia y se selecciona un activo. | La lista, el grafo y el inspector muestran datos del fixture y su evidencia; no se atribuyen a un objetivo real. |
| Importación archivada | Está seleccionado el fixture positivo y el dominio consultado es `example.com`. | Se importa el archivo urlscan desde disco. | La terminal presenta el resultado, el grafo muestra pistas históricas con hash/linaje y el Snapshot permanece separado. |
| Control negativo | Está disponible el fixture urlscan negativo. | Se procesa con la misma raíz `example.com`. | Los candidatos externos a la raíz no aparecen como activos CHEF. |

Registrar cada criterio como **aprobado**, **fallido** o **no ejecutado**, con fecha, revisión y versión del instalador. En esta pasada, la exploración completa, importación urlscan y control negativo permanecen **no ejecutados**. No inferir éxito en estas rutas a partir de la prueba de terminal.

## Guion de cinco minutos

| Tiempo | Acción | Evidencia que se puede afirmar |
| --- | --- | --- |
| 00:00–00:35 | Presentar CHEF como estación local de evidencia para reconocimiento autorizado. | La app funciona offline; importar datos no autoriza tráfico. |
| 00:35–01:20 | Abrir la aplicación instalada y revisar el snapshot sintético de inicio. | Mostrar la lista de activos, el mapa de relaciones y la terminal local. Identificar datos sintéticos. |
| 01:20–02:05 | Seleccionar un activo y abrir su inspector. | Recorrer evidencia y procedencia; distinguir observación de inferencia. No afirmar vulnerabilidad por un puerto o una relación. |
| 02:05–03:20 | Ir a Importaciones, indicar `example.com`, seleccionar el fixture urlscan positivo y procesarlo. | Mostrar fases en la terminal y guardar el grafo en SurrealDB local. Los resultados son pistas históricas con tiempo fuente desconocido, no DNS actual. |
| 03:20–04:10 | Revisar el grafo del archivo importado y su hash SHA-256. Si el recorrido es estable, repetir con el fixture negativo. | Los hosts externos a la raíz no se proyectan como candidatos. El Snapshot no se reemplaza con datos históricos de proveedor. |
| 04:10–05:00 | Volver a Evidencia y resumir límites y siguiente incremento. | La importación URLscan/Censys compone parser Rust, IPC y almacenamiento; fuentes remotas, escaneo activo, recuperación/backup multiplataforma y portado total siguen pendientes. |

## Responsabilidades del ensayo

- **César:** explicar límites de importación, contrato, IDs deterministas y por qué una pista no concede scope.
- **Diego:** conducir la navegación, lista, grafo, inspector y terminal; observar accesibilidad y legibilidad.
- **Jhojan:** explicar persistencia local, hash/linaje, fixtures positivos/negativos y recuperación del almacén.

Los tres separan en su explicación lo implementado, lo validado automáticamente y lo comprobado manualmente. La revisión automatizada no sustituye la revisión humana del equipo.

## Recuperación y límites

Si el instalador o la ventana WebView no abren, usar `npm run web:dev` como fallback visual offline y etiquetarlo claramente como navegador de desarrollo, no como instalador. Si falla la importación urlscan, mostrar el snapshot local y el resultado de pruebas automatizadas; no improvisar con una consulta real. No usar datos de clientes, secretos, API keys ni objetivos reales en capturas.

La CLI offline (`npm run demo`) es una alternativa de respaldo para explicar la validación del núcleo. No demuestra la UI Tauri, SurrealDB, el instalador ni un flujo de reconocimiento real. Las capacidades del portado siguen pendientes según la [auditoría](../migration/auditoria-fuente.md), el [inventario](../migration/inventario-migracion.csv) y el [plan Rust](../migration/plan-migracion-rust.md).
