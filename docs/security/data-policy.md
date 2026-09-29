# Datos y retención

El incremento actual no tiene base de datos, telemetría ni sincronización. Mantiene metadata/hash/locator durante ejecución; raw permanece solo en el archivo local elegido por el usuario. La exportación contiene direcciones/servicios y puede identificar infraestructura del cliente; el propietario controla dónde guardarla.

No subir XML real, headers, tokens, cookies, URLs sensibles, exportaciones ni capturas de tráfico a GitHub. Todos los fixtures del repo son sintéticos; scripts/logs no deben incluir credenciales ni paths de disco. La huella SHA-256 identifica evidencia y no equivale a cifrado ni anonimización.

Futuro HTTP: eliminar auth/cookies/body/query por defecto; paths también pueden incluir identificadores sensibles, por lo que hash de path por defecto o consentimiento por proyecto para mostrar paths. Export preview muestra campos incluidos. La [persistencia web propuesta](../adr/0008-web-platform.md) usa PostgreSQL/Prisma y requiere borrado por proyecto, trazabilidad de borrado y retención configurable (propuesta inicial: 7 días para raw opt-in, pendiente de decisión). Evaluar cifrado de volumen, secretos, backups y gestión de claves antes de almacenar raw; PostgreSQL por sí solo no resuelve esa política. SQLite permanece como alternativa histórica del [ADR 0004](../adr/0004-persistence.md), no como objetivo web vigente.

En la demo base se usan loopback y fixtures inventados; cualquier piloto con fuente pasiva real exige dominio/dataset autorizado y términos de uso registrados. Las capturas públicas no llevan dominios reales ni identidad de terceros. Crear evidencia auditada con input digest, parser/schema/rules/scope digest y timestamp. Nunca guardar secretos en el work log.
