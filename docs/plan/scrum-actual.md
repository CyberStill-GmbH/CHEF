# Scrum vigente de CHEF

Actualizado: 27-10-2026. Este plan organiza el cierre de la vertical de escritorio offline y el avance progresivo del backend Rust entre las tres personas del equipo. La revisión manual de WebView, la suite funcional y la conformidad completa siguen pendientes; el propietario aplazó las pruebas funcionales. El [ADR 0012](../adr/0012-escritorio-tauri-rust.md) define arquitectura, límites y criterios de aceptación.

## Objetivo de producto

Entregar una aplicación local CHEF que permita importar evidencia JSONL desde archivo, aplicar una política de alcance de importación, derivar activos tipados con procedencia, persistir el snapshot y el grafo local, y revisar la actividad del proceso en la interfaz. La implementación objetivo es React/Vite/TypeScript, Tauri v2, Rust/Tokio y SurrealDB embebida sobre RocksDB. La demo permanece offline: importar datos no autoriza tráfico y no se ejecutan consultas DNS/HTTP ni integraciones remotas.

## Estado comprobable

- **Implementado en el checkout:** shell de escritorio, importación local acotada, parser y proyección Rust, persistencia local, exportación JSON, eventos de actividad y componentes React para revisar el grafo y la evidencia.
- **Verificado automáticamente:** `cargo check --locked --offline` para el crate de escritorio, `npm run check:docs` y comprobaciones estáticas registradas en el progreso. El build de escritorio depende de toolchain nativo disponible en el equipo.
- **Pendiente de revisión:** abrir la aplicación en WebView, recorrer importación/consulta/exportación, verificar recuperación y backup del almacén, y revisión humana del equipo.
- **Aplazado por el propietario:** pruebas funcionales y reconstrucción de paridad con la suite de origen.
- **Fuera del corte:** escaneo activo, consultas a proveedores, envío de datos a servicios externos y sincronización remota.

El portado completo del motor fuente sigue incompleto. La [auditoría](../migration/auditoria-fuente.md), el [plan de migración](../migration/plan-migracion-rust.md) y el [inventario](../migration/inventario-migracion.csv) muestran el estado por componente. Solo se considera terminado lo que esté integrado, compilado y con límites CHEF documentados; la revisión funcional queda aplazada hasta que el propietario la habilite.

## Capacidad y coordinación

César, Diego y Jhojan disponen de hasta 10 horas semanales de referencia por persona. En planning, el equipo confirma la disponibilidad real y ajusta el alcance. Cada integrante mantiene una historia principal activa, explicita dependencias y solicita revisión a alguien distinto de quien implementó.

El Daily Scrum dura hasta 15 minutos. Cada persona responde: qué resultado comprobable produjo, cuál es el siguiente paso de hasta dos horas, qué bloqueo tiene y qué revisión necesita. La review separa compilación, validación automática, recorrido manual y propuesta; compilar no equivale a validar el flujo de usuario.

## Sprint actual · cierre de la vertical offline · 27-31 de octubre

**Objetivo:** dejar un recorrido de importación offline entendible y revisable, cerrar defectos de integración del backend Rust y registrar con claridad lo que todavía requiere pruebas o revisión manual.

| Responsable                         | Prioridad                                                                                                                                                                                                                                | Criterio de aceptación                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| César · núcleo e integración        | Completar módulos Rust locales que no requieran red; integrar parser, políticas de límites, jobs Tokio y comandos Tauri; mantener fallos tipados y evitar que el scope de importación se interprete como permiso de tráfico.             | Dado un archivo JSONL y una política CHEF válida, cuando Tauri procesa la importación, entonces persiste solo un resultado validado y transmite estado/progreso sin bloquear la interfaz. El módulo compila; la conformidad funcional queda pendiente. Diego revisa interfaz del comando y Jhojan revisa persistencia.                                            |
| Diego · interfaz e IPC              | Consolidar los nombres CHEF de la experiencia, presentar importación, estado del trabajo, grafo, evidencia y transcripción accesible; validar los DTO IPC antes de mostrarlos como texto.                                                | Dado un resultado de importación disponible, cuando la persona lo abre, entonces puede distinguir datos observados, pistas y candidatos, y consultar el estado desde una lista accesible. La integración compila; el recorrido manual en WebView se registra como pendiente hasta ejecutarse. César revisa contratos IPC y Jhojan revisa la lectura de evidencia. |
| Jhojan · persistencia y conformidad | Revisar el ciclo del almacén SurrealDB/RocksDB, identidad determinista, aislamiento por proyecto, relectura del snapshot y requisitos de backup/recuperación; mantener fixtures y documentos de trazabilidad alineados con lo integrado. | Dado un snapshot validado, cuando se guarda y vuelve a leer, entonces conserva identidad, evidencias, procedencia y relaciones sin presentar un guardado parcial como completo. El build de storage pasa; recuperación/backup manual queda explícitamente pendiente. César revisa límites de importación y Diego revisa la vista de evidencia.                    |

## Definition of Done

Una historia se cierra cuando satisface su criterio Dado/Cuando/Entonces, compila, respeta la arquitectura por capas y la revisa otra persona. Los cambios de contrato requieren esquema, ejemplo y nota de compatibilidad; las pruebas funcionales permanecen aplazadas hasta indicación del propietario. El registro de avance diferencia siempre implementación, verificación automática y validación manual.

No se amplía el alcance desde datos importados. Ninguna historia de este sprint activa red, fuentes externas ni exportación remota. Si aparece una dependencia de autorización, licencia, scope o evidencia, se registra como bloqueo de producto y se mantiene el comportamiento offline.

## Referencias operativas

- [Backlog del escritorio](backlog-desktop.md)
- [Daily Scrum](daily-scrum.md)
- [Equipo y responsabilidades](team.md)
- [PRD de escritorio](../product/desktop-prd.md)
- [Requisitos de escritorio](../product/desktop-requirements.md)
- [Progreso verificable](../process/progress.md)
