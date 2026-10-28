# Requisitos de la aplicación de escritorio CHEF

Actualizado 18-10-2026. Este documento describe el destino Tauri/Rust solicitado. **Ningún requisito de escritorio se considera implementado hasta que exista código y evidencia**; el estado actual está en [alcance](scope.md) y los pasos en el [backlog de migración](../plan/backlog-desktop.md).

| ID      | Resultado observable                                                               | Estado actual                                                                           | Verificación requerida                                                                           |
| ------- | ---------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| DESK-01 | La aplicación abre una ventana Tauri v2 con frontend React/Vite/TypeScript         | Parcial: `cargo check` pasó; bundle y flujo WebView pendientes                          | Instalación y build por plataforma                                                               |
| DESK-02 | El usuario inicia una tarea con un comando IPC tipado y recibe un `jobId`          | Propuesto                                                                               | Solicitud válida, inválida y límite de tamaño                                                    |
| DESK-03 | El backend ejecuta trabajos Tokio sin bloquear la interfaz y permite cancelarlos   | Propuesto                                                                               | Cancelación antes/durante trabajo y prueba de respuesta UI                                       |
| DESK-04 | La terminal xterm.js representa logs `scan-log` estructurados en orden             | Parcial: bridge Tauri/React, validación y transcripción accesible; falta WebView        | Secuencia, lote, mensaje hostil, evento duplicado y tarea finalizada                             |
| DESK-05 | La demo predeterminada procesa solo fixtures y deja claro que es simulación        | Parcial: importación de fixture y terminal offline conectadas; falta validar shell      | Sin DNS/socket/HTTP; captura de UI y registro del flujo                                          |
| DESK-06 | Se importa y exporta `Snapshot 1.0.0` conservando identidad y evidencia            | Parcial: Tauri exporta JSON Snapshot validado; golden pendiente                         | Golden TS/Rust, orden estable, referencias y compatibilidad                                      |
| DESK-07 | SurrealDB embebida con RocksDB persiste snapshots locales con IDs deterministas    | Parcial: escritura, lectura y export en Tauri pasan compilación                         | Crear, cerrar, reabrir, recuperar y exportar; compatibilidad de esquema y backup multiplataforma |
| DESK-08 | La configuración guarda preferencias sin secretos                                  | Parcial: loader TOML local, límites y DTO sin credenciales; edición/guardado pendientes | Inspección de TOML, logs y errores; secreto ausente en claro                                     |
| DESK-09 | Los conectores pasivos solo consultan fuentes habilitadas y autorizadas            | Bloqueado por permiso                                                                   | Aprobación de términos, cuenta, consentimiento, cuota, locator y fecha                           |
| DESK-10 | DefectDojo recibe una exportación solo tras vista previa y confirmación explícitas | Propuesto, integración desactivada                                                      | Confirmación, redacción, error de red y ausencia de token en log                                 |
| DESK-11 | El escaneo activo DNS/HTTP está desactivado hasta registrar autorización y alcance | Regla vigente; módulo futuro                                                            | Sin scope: cero tráfico; con laboratorio aprobado: límites/cancelación/auditoría                 |
| DESK-12 | La lista permite la misma lectura que el grafo cuando se implemente                | Parcial en React web; grafo futuro                                                      | Teclado, lector de pantalla, relaciones y evidencias equivalentes                                |

## Criterios de aceptación iniciales

**Dado** que la aplicación abre el fixture canónico, **cuando** el usuario selecciona un activo, **entonces** puede seguir `Asset → Observation → Evidence` y ver fecha, origen e integridad.

**Dado** que el usuario inicia la simulación offline, **cuando** la tarea avanza, **entonces** llegan mensajes estructurados con secuencia y estado, la interfaz permanece receptiva y cancelar detiene el trabajo.

**Dado** un XML malicioso, referencia inexistente o snapshot de versión desconocida, **cuando** se importa, **entonces** la operación falla sin persistir resultado parcial ni cambiar IDs existentes.

**Dado** que el usuario no ha autorizado consultas o exportaciones remotas, **cuando** abre la aplicación, **entonces** no sale tráfico hacia objetivos, proveedores OSINT ni DefectDojo.

**Dado** una relación por hostname/IP compartidos o evidencia histórica, **cuando** se muestra el grafo, **entonces** permanece candidata o conflictiva y no se marca como propiedad ni vulnerabilidad confirmada.

## Requisitos de calidad y despliegue

- Rust usa tipos y puertos explícitos; filesystem, Tauri, SurrealDB y red viven en adaptadores.
- Los límites del trabajo asíncrono, cola de logs, respuestas y persistencia son configurables con valores máximos seguros; no se ejecuta en concurrencia ilimitada.
- Capacidades Tauri solo exponen comandos requeridos a la ventana principal. No habilitar shell ni acceso general a filesystem.
- La base usa el directorio de datos de la aplicación. Copia, restauración, retención y migración se documentan antes de guardar evidencia real.
- El paquete se construye por sistema operativo y declara runtime WebView y dependencias nativas.
- `Snapshot 1.0.0` mantiene su semántica. Cualquier cambio wire requiere versión, esquema, ejemplo, prueba y nota de compatibilidad.
