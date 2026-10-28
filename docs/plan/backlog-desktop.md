# Backlog de migración a escritorio CHEF

Actualizado 18-10-2026. Este es el backlog vigente para la dirección Tauri/Rust aceptada por el propietario. El [backlog web anterior](backlog.md) se conserva como antecedente; sus historias Node/Go y PostgreSQL/Prisma no son trabajo activo. Las estimaciones son de refinamiento inicial y deben ajustarse al completar Tauri y las pruebas SurrealDB/RocksDB multiplataforma.

## Estados y reglas

- **Propuesto:** dirección aceptada por el propietario, pendiente de refinamiento/revisión del equipo.
- **Listo:** responsable, revisor, dependencia y criterios Dado/Cuando/Entonces definidos.
- **Bloqueado:** falta permiso, herramienta o decisión externa.
- **Parcial:** existe una parte del resultado; el criterio completo sigue pendiente.
- **Hecho:** código, pruebas pertinentes, revisión distinta del autor y evidencia verificable.

No se ejecuta descubrimiento activo ni se envían datos a terceros sin autorización específica. `Snapshot 1.0.0`, IDs deterministas, procedencia, control negativo, errores atómicos y ausencia de falsos claims son condiciones de aceptación.

## Historias priorizadas

| ID   | Resultado                                                                                    | Prioridad/estado                                                                                              | Responsable → revisor  |        Estimación | Dependencias                                 |
| ---- | -------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------- | ---------------------- | ----------------: | -------------------------------------------- |
| TD00 | Fijar procedencia y licencia de origen para CHEF                                                  | P0/resuelto: árbol tratado íntegro como de origen; AGPL-3.0-only adoptada                                          | César → Jhojan         |             hecho | inventario y decisión del propietario        |
| TD01 | Preparar toolchain Rust/Tauri y build de ejemplo reproducible                                | P0/parcial: almacenamiento y shell pasan `cargo check`; falta bundle                                          | César → Jhojan         |             2–4 h | dependencias nativas de plataforma           |
| TD02 | Completar el crate Rust para leer/escribir snapshots y coincidir con golden TS               | P0/parcial: Nmap, JSONL de origen, proyección CHEF y Snapshot 1.1                                                  | César → Jhojan         |            8–14 h | TD01                                         |
| TD03 | Completar tarea Tauri offline con `jobId`, cancelación y logs `scan-log` acotados            | P0/parcial: canal Tokio→Tauri conectado; faltan cancelación UI, límites/agrupación y validar compilación      | César → Diego          |            6–10 h | TD01, DTO de logs                            |
| TD04 | Integrar xterm.js con el explorador React y representar logs tipados en modo texto           | P0/parcial: `listen`, validación, xterm.js y transcript; faltan verificación build y prueba manual en WebView | Diego → César          |            6–10 h | TD03                                         |
| TD05 | Completar SurrealDB local/RocksDB: esquema de grafo, compatibilidad, backup y concurrencia   | P0/parcial: persiste snapshot/origen/grafo; falta validar Tauri y backup                                      | Jhojan → César         |             4–8 h | TD01                                         |
| TD06 | Importar fixture de origen y persistir activos/evidencias con IDs CHEF; presentar grafo en la UI  | P1/parcial: importación, persistencia, diagrama y lista accesible implementados                               | César + Jhojan → Diego |            8–14 h | TD02, TD05                                   |
| TD07 | Configuración TOML sin secretos y adaptador a almacén seguro del sistema operativo           | P1/parcial: loader Tauri y DTO versionado; falta interfaz de edición y almacén seguro                         | César → Jhojan         |             4–8 h | TD01, elección de crate/plugin               |
| TD08 | Conector pasivo de un proveedor con permiso, cuota, fecha, locator y estado de error         | P2/bloqueado por permisos                                                                                     | Jhojan → César         | 8–16 h por fuente | términos, credencial y scope autorizados     |
| TD09 | Exportación explícita a DefectDojo con vista previa y confirmación                           | P2/propuesto                                                                                                  | César → Jhojan         |            6–12 h | contrato de export, instancia de laboratorio |
| TD10 | Descubrimiento activo DNS/HTTP con autorización, límites, cancelación y auditoría            | P3/bloqueado por scope                                                                                        | César → Jhojan         |       por estimar | nuevo ADR y laboratorio autorizado           |
| TD11 | Empaquetado, instalación, actualización, backup/restauración y validación manual por sistema | P1/propuesto                                                                                                  | Diego + César → Jhojan |            8–16 h | TD02–TD06, entornos Windows/macOS/Linux      |
| TD12 | Portar el catálogo completo de origen a módulos Rust CHEF con destino o exclusión por archivo     | P0/parcial: JSONL, proyección, presets declarativos y agregado offline; catálogo pendiente                    | César + Jhojan → Diego |       por refinar | inventario de origen y contratos CHEF             |

## Criterios Dado/Cuando/Entonces

**TD02 — conformidad.** Dado el fixture canónico y el mismo alcance, cuando el core Rust procesa el snapshot, mantiene los IDs, relaciones y evidencias de TypeScript. Dado XML malicioso o referencias inválidas, falla sin publicar una salida parcial.

**TD03/TD04 — tarea y terminal.** Dada una tarea de simulación offline, cuando se inicia, la UI recibe mensajes tipados y secuenciados sin bloquear. Cuando el usuario cancela, no se emiten mensajes posteriores de esa tarea. El simulador no crea tráfico de red.

**TD05/TD06 — persistencia.** Dado un snapshot válido, cuando se guarda, se puede cerrar/reabrir y exportar con los mismos IDs. Dado un cierre durante escritura, el próximo inicio no muestra el run como completo si faltan datos o referencias.

**TD08–TD10 — límites externos.** Dada una fuente o destino no autorizados, cuando se abre el proyecto, no hay peticiones. Dado un envío remoto confirmado, el usuario ve proveedor, destino y contenido, puede cancelar antes de transmitir y recibe un resultado rastreable.

## Corte del 31-10

El objetivo realista es una primera vertical local con compilación Tauri, explorador del fixture, tarea simulada y decisión respaldada sobre el motor de almacenamiento. TD08, TD09 y TD10 no son requisitos de demo hasta pasar sus gates. No prometer base de datos migrada, fuente viva, envío a DefectDojo ni escaneo activo por tener una interfaz dibujada.
