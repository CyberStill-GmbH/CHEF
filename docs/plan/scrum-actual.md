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

**Objetivo del sprint/presentación:** validar en Windows un recorrido offline de escritorio que abra el snapshot local, permita explorar activos/evidencia y procese una respuesta urlscan archivada en Rust/SurrealDB, sin tráfico ni secretos. Registrar aparte build, pruebas automáticas, instalación y recorrido manual; el portado completo del backend continúa fuera de este sprint.

| Responsable                         | Prioridad                                                                                                                                                                                                                                | Criterio de aceptación                                                                                                                                                                                                                                                                                                                                            |
| ----------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| César · núcleo e integración        | Portar parsers archivados de manera incremental; conectar URLscan al caso de uso Tokio y los comandos Tauri; mantener límites, linaje y errores sin habilitar transporte remoto.                                                   | Dada una respuesta urlscan sintética y un dominio raíz, cuando se importa en escritorio, entonces Rust produce IDs/hash/linaje deterministas, la guarda separada del Snapshot y emite fases de actividad; nunca genera tráfico. Diego revisa IPC y Jhojan persiste/recupera el grafo.                                    |
| Diego · interfaz e IPC              | Entregar el shell CHEF oscuro, navegación, lista de activos, mapa, inspector y terminal; importar y mostrar fixtures sin atribuirles actualidad ni autoridad.                                                                             | Dado el snapshot inicial o una respuesta archivada, cuando se explora, entonces la persona puede distinguir observaciones de pistas, abrir el inspector y volver al snapshot. El test web corre automáticamente; el recorrido manual instalado queda registrado por separado. César revisa el contrato y Jhojan revisa la lectura.         |
| Jhojan · persistencia y conformidad | Validar upsert/lectura de importaciones de proveedor en SurrealDB/RocksDB, IDs y linaje; preparar instalación/recuperación de la demo y mantener el inventario documental sincronizado.                                                   | Dada una respuesta válida, cuando se guarda y vuelve a leer, entonces conserva dominio, hash, eventos, proyección e identidad sin presentarse como Snapshot actual. Registrar el build MSI/NSIS y prueba de instalación; el backup/recuperación multiplataforma queda pendiente hasta validarse. César revisa límites y Diego revisa la UI. |

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
