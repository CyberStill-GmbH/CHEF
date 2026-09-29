# Datos y retención

El incremento actual no tiene base de datos, telemetría ni sincronización. Mantiene metadata/hash/locator durante ejecución; raw permanece solo en el archivo local elegido por el usuario. La exportación contiene direcciones/servicios y puede identificar infraestructura del cliente; el propietario controla dónde guardarla.

No subir XML real, headers, tokens, cookies, URLs sensibles, exportaciones ni capturas de tráfico a GitHub. Todos los fixtures del repo son sintéticos; scripts/logs no deben incluir credenciales ni paths de disco. La huella SHA-256 identifica evidencia y no equivale a cifrado ni anonimización.

Futuro HTTP: eliminar auth/cookies/body/query por defecto; paths también pueden incluir identificadores sensibles, por lo que hash de path por defecto o consentimiento por proyecto para mostrar paths. Export preview muestra campos incluidos. Toda persistencia tendrá borrado por proyecto, trazabilidad de borrado y retención configurable (propuesta: 7 días para raw opt-in, no valor obligatorio). SQLite no proporciona cifrado automáticamente; evaluar sistema/volumen cifrado y key management antes de almacenar raw.

En demo solo loopback/fixtures inventados; capturas públicas sin dominios reales ni identidad de terceros. Crear evidencia auditada con input digest, parser/schema/rules/scope digest y timestamp. Nunca guardar secretos en el work log.
