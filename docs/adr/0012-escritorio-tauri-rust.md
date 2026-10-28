# ADR 0012: Aplicación de escritorio Tauri con núcleo Rust

Estado: **dirección de producto aceptada por el propietario; build y bundles MSI/NSIS generados en Windows; revisión técnica del equipo y validación manual pendientes**. Fecha: 18-10-2026. Responsables propuestos: César (núcleo Rust/IPC), Diego (React/UX/terminal) y Jhojan (persistencia, fixtures y conformidad). Define el destino que reemplaza la propuesta web Node–Go del [ADR 0011](0011-node-go-boundary.md); no elimina el código ni los registros históricos.

## Contexto y decisión que se debe tomar

El checkout contiene un núcleo/CLI TypeScript offline, un explorador React/Vite que lee un fixture local, un spike Java/Montoya y ahora un primer crate `packages/rust-core` que importa el fixture Nmap y conserva los IDs de TypeScript en pruebas. No contiene backend Python, módulo Go ni aplicación Tauri. La dirección anterior de producto web multiusuario no corresponde a la instrucción actual del propietario, que pide una aplicación de escritorio offline-first para analistas.

El propietario seleccionó React, Vite, TypeScript, Tailwind CSS y shadcn/ui en el frontend; Tauri v2 para la ventana e IPC; Rust con Tokio para casos de uso, trabajos y adaptadores; **SurrealDB embebida sobre RocksDB** para persistencia y grafo; y una integración opcional con DefectDojo. La terminal usa xterm.js como presentación de logs estructurados. Shodan, Censys y VirusTotal son candidatos de fuentes pasivas sujetos a credenciales, términos y permiso. El escaneo activo DNS/HTTP queda diseñado como fase futura y desactivada hasta que exista alcance escrito y controles aprobados.

La carpeta local indicada por el propietario, `C:\projects\CHEF`, contiene la fuente de origen 3.0.2 bajo AGPL. El propietario decidió migrar a Rust las capacidades que sirvan a CHEF y adoptar AGPL-3.0-only. La migración sigue por etapas. El primer adaptador CHEF importa offline eventos JSONL de puertos abiertos bajo Snapshot 1.1.0, aplica scope de importación y conserva procedencia. Véanse la [auditoría y el plan de migración](../migration/auditoria-fuente.md) y el [inventario por archivo](../migration/inventario-migracion.csv).

## Impulsores y opciones

Importan el funcionamiento offline, la instalación local, el aislamiento entre interfaz y capacidades del sistema, la conservación de IDs/evidencia, la cancelación, la trazabilidad y un alcance de red verificable. Las opciones son mantener solo el incremento TypeScript; migrar el core y la composición a Rust/Tauri; o mantener una API Node con motor Go y añadir una UI de escritorio encima. El propietario eligió la segunda opción para el destino. La decisión aún requiere revisión técnica del equipo y una prueba de compilación/almacenamiento en los sistemas operativos objetivo.

## Decisión aceptada y justificación

Adoptar una aplicación de escritorio Tauri v2. React sigue siendo una capa de presentación; el código Rust es la autoridad para validación, identidad, reglas, alcance, trabajos, persistencia y sincronización. Tokio ejecuta trabajos acotados fuera del hilo de interfaz. Los comandos IPC aceptan solicitudes tipadas y devuelven un identificador de trabajo; un canal interno acotado comunica resultados, cancelación y errores. Los logs se envían al webview principal como eventos `scan-log` con un payload versionado y estructurado, en lotes con tamaño y frecuencia limitados. El frontend valida el payload y lo representa en xterm.js como texto; no ejecuta contenido recibido.

La decisión vigente es **SurrealDB embebida con RocksDB**, abierta una vez desde Rust en el directorio de datos que devuelve Tauri. `packages/rust-storage` usa el SDK Rust 3.3.4, namespace/database CHEF locales y un registro por snapshot validado bajo su `runId`; el adaptador persiste también los eventos fuente y el grafo CHEF tipado. En Windows se instalaron NASM 3.02, CMake 4.4.4 y libclang 18.1.1 en el perfil local del usuario tras verificar los SHA-256; `cargo check`, `cargo build --locked` y el empaquetado MSI/NSIS pasan. Los bundles están en `apps/desktop/src-tauri/target/release/bundle`; el arranque WebView y el flujo manual siguen pendientes. Se deben validar migraciones SurrealQL, backup/restore, cierre inesperado, bloqueo del directorio, tamaño del instalador y compatibilidad multiplataforma. La búsqueda vectorial se añade solo si las métricas lo justifican.

El [diseño de persistencia local](../architecture/persistencia-local.md) documenta SurrealDB/RocksDB como decisión, las alternativas evaluadas y el vector futuro. Los vectores no son parte de identidad ni crean relaciones. SurrealDB 3.3.4 declara el núcleo bajo BSL 1.1 con Additional Use Grant y el SDK por separado; se deben conservar y revisar avisos/licencias de la versión fijada. MongoDB local requiere más piezas de proceso/servicio y no se adapta al instalador offline de escritorio.

El archivo `config.toml` guarda preferencias y habilitación de conectores, nunca claves API ni tokens. Los secretos se obtienen de un almacén seguro del sistema operativo mediante un adaptador aún por elegir. DefectDojo permanece desactivado por defecto: cada exportación remota requiere acción explícita, vista previa del proyecto/datos, destino y engagement; la respuesta y el resultado de sincronización conservan un ID local y estado trazable.

La primera versión de escritorio debe demostrar únicamente el flujo offline con fixtures: abrir un snapshot, consultar activos/evidencia, persistirlo localmente y mostrar logs de simulación. No implementa ni ejecuta descubrimiento DNS/HTTP activo. Las fuentes pasivas y el cliente DefectDojo se integran por adaptadores con autorización separada, sin enviar dominios o datos de clientes por defecto. La UI no obtiene acceso directo a filesystem, base, red, shell ni claves; las capacidades de Tauri se limitan a comandos explícitos y cada comando vuelve a validar sus entradas.

## Consecuencias y migración

- Se conserva React/TypeScript para la interfaz. Tauri v2 compone la importación offline y xterm.js presenta eventos de actividad versionados; Tailwind y shadcn/ui aún deben incorporarse con versiones fijadas y revisión de licencias.
- El núcleo TypeScript sigue siendo la referencia ejecutable durante la migración. El primer crate Rust ya reproduce el fixture Nmap y los IDs golden; debe ampliar la conformidad a casos de borde y lectores de snapshot antes de sustituir el uso de producción. No se declara que existe código Python para portar en este checkout.
- `Snapshot 1.0.0` permanece sin cambios como formato interoperable y golden de compatibilidad. Los IDs deterministas, `Observation → Evidence`, control negativo, errores atómicos y límites del alcance son invariantes. Si el modelo local multifuente exige campos o semántica nueva, crear versión mayor con esquema, ejemplo, prueba y nota de compatibilidad antes de publicarla.
- La base embebida no implica cifrado, backup ni protección automática de datos. Definir permisos del directorio, política de retención, copia/restauración y migraciones antes de almacenar evidencia real.
- La terminal muestra eventos estructurados, no una shell interactiva. La palabra “escaneo” en la demo identifica un simulador offline, hasta aprobar una capacidad de red independiente.
- El instalador se genera por plataforma objetivo y depende del motor WebView nativo del sistema; no se promete un único binario universal para Windows, macOS y Linux.

Fases: (1) preservar el estado actual y crear fixtures compartidos; (2) crear el núcleo Rust y reproducir snapshots; (3) persistir snapshots en SurrealDB/RocksDB; (4) conectar el importador local con Tauri y React; (5) transmitir actividad por Tokio y `scan-log` —implementado parcialmente, con revisión y recorrido manual pendientes—; (6) normalizar el grafo y validar recuperación; (7) decidir fuentes pasivas y DefectDojo con permisos; (8) considerar escaneo activo solo con una autorización nueva, límites, cancelación, auditoría y laboratorio. Las pruebas funcionales quedan para la fase posterior solicitada por el propietario. La CLI TypeScript se mantiene como referencia hasta demostrar conformidad.

## Verificación

1. Dado el fixture Nmap vigente, cuando el core Rust lo procesa con el mismo alcance, produce los mismos IDs, relaciones, evidencia y `Snapshot 1.0.0` que TypeScript.
2. Dado un hostname compartido, estado `open|filtered`, evidencia histórica o XML malicioso, cuando Rust procesa el fixture, conserva el control negativo y no declara propiedad ni vulnerabilidad confirmada.
3. Dado un comando de simulación, cuando la UI inicia y cancela el trabajo, recibe logs con secuencia/fecha/estado, la cancelación detiene la generación y la interfaz no se bloquea.
4. Dado un cierre inesperado durante persistencia, cuando se reinicia la aplicación, no presenta una importación parcial como completa y registra el error sin payload sensible.
5. Dada una solicitud DefectDojo sin habilitación o sin confirmación explícita, cuando se intenta sincronizar, no se transmite ningún dato.
6. Dado un objetivo DNS/HTTP sin autorización y alcance escritos, cuando se abre la función activa, permanece desactivada y no inicia tráfico.

La validación requiere `cargo test` para crates Rust, `npm run check` para interfaz/contratos, checks de empaquetado por sistema operativo y revisión manual del flujo Tauri. La revisión manual de DNS/HTTP no se realiza hasta tener autorización y entorno de laboratorio.

## Fuentes y revisión

- [Comandos, eventos y canales de Tauri v2](https://v2.tauri.app/develop/calling-rust/) — consultado el 18-10-2026. La documentación recomienda canales para flujos continuos; el evento `scan-log` se limitará y agrupará.
- [Capacidades de Tauri v2](https://v2.tauri.app/security/capabilities/) — consultado el 18-10-2026.
- [SurrealDB embebido en Rust](https://surrealdb.com/docs/reference/rust/embedding) y [motores de almacenamiento](https://surrealdb.com/docs/build/embedding/storage-engines) — consultados el 18-10-2026.
- [SDK Rust embebido y RocksDB](https://surrealdb.com/docs/reference/rust/embedding), [motores de almacenamiento](https://surrealdb.com/docs/build/embedding/storage-engines) y [licencias de SurrealDB](https://surrealdb.com/license) — consultados el 18-10-2026; fuentes detalladas en [sources.json](../research/sources.json).

Las fuentes quedan registradas en [sources.json](../research/sources.json). La dirección de producto fue aceptada por el propietario; falta la revisión humana de César, Diego y Jhojan sobre viabilidad y prioridades antes de retirar componentes TypeScript.
