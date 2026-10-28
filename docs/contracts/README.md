# Contratos

La prioridad web se discute en el [borrador de API multiusuario](web-api-proposal.md). La [API loopback](api.md) y los [eventos SSE](events.md) son diseños históricos no implementados; **no son contratos estables**. Los formatos ejecutables actuales son Snapshot 1.0.0 para Nmap y Snapshot 1.1.0 para la ingestión offline de BBOT, además del envelope HTTP separado del spike Montoya.

`snapshot.schema.json` es la autoridad wire del incremento offline, JSON Schema 2020-12. Mantiene snapshots Nmap `1.0.0` y añade `1.1.0` para evidencia importada desde BBOT JSONL; los tipos del dominio deben concordar mediante pruebas. `Asset`, `Evidence`, `Observation`, `Relationship`, `ExposurePath`, `ScanRun` y `ScopePolicy` tienen campos requeridos y propiedades extra prohibidas. `Finding` está definido como contrato reservado; el motor actual no lo produce.

## Semántica

- Asset: entidad estable; firstSeen/lastSeen son tiempos observados, no reloj de importación; ≥1 Observation.
- Observation: hecho `open-port`, source/version/run y locator dentro del archivo; una Evidence; timestamps UTC RFC3339. `1.0.0` acepta `nmap-xml`; `1.1.0` acepta `bbot-ndjson`.
- Evidence: SHA-256 de bytes, parserVersion, importedAt y locator raíz. No incorpora raw sensible ni ruta de filesystem. Reimportar idénticos bytes/scope/versión produce los mismos IDs.
- Relationship: exactamente dos Asset existentes; ≥1 Observation existente, assertion observed.
- ExposurePath: en v1 exactamente dos assets y una relación; assertion inferred, regla open-service/v1, vulnerabilidad confirmada siempre false.
- ScanRun: snapshot publicado únicamente al completarse; IDs deterministas por bytes/scope/parser. La API futura manejará intentos cancelados/fallidos con IDs independientes.
- ScopePolicy: allowlist de IP/CIDR, exclusiones con precedencia, puertos/protocolos y límites. Importar un archivo no autoriza tráfico ni modifica la policy.
- Finding reservado: necesita evidence vía Observation y revisión humana; todavía sin API/implementación. Un puerto abierto no genera Finding.

Confianza 0..1: en v1 describe fidelidad al reporte importado. No está calibrada estadísticamente. La fusión usa máximo sin interpretar duplicados como evidencia independiente. Conservar contradicciones/estado closed en futuras versiones requiere nuevo tipo de Observation.

## Identidad y fusión

SHA-256 de JSON de una lista ordenada de partes UTF-8; prefijo asset/observation/evidence/run/relationship/path. No concatenar campos ambiguos sin delimitación. Keys address/service se documentan en SOLID. Scope normaliza y ordena sus arrays antes de hashing. Golden fixture fija IDs para evitar divergencia Java/TS.

Schema verifica forma, no integridad referencial ni autoría. Pruebas del pipeline verifican referencias; importación de snapshots externos requerirá un verificador semántico antes de persistirlos. Los exporters actuales solo emiten resultados del pipeline validado.

## Ejemplos y compatibilidad

`event-statistics.schema.json` define el DTO IPC `1.0.0` que acompaña una importación local BBOT: conteos deterministas por tipo y productor. El comando no modifica `Snapshot 1.1.0` y su respuesta es independiente; clientes que no consumen el comando siguen usando snapshots sin cambios. La vista no es un reporte de ejecución en vivo: los contadores de consumo y velocidad requieren un ciclo de ejecución que aún no se integra en Tauri. El ejemplo contiene los tres eventos sintéticos del fixture CHEF.

[Scope ejecutable](../../fixtures/scope/lab.json), [Nmap sintético](../../fixtures/nmap/lab.xml), [eventos JSONL de importación](../../fixtures/event-import/events.ndjson), [snapshot ejemplo 1.0](examples/snapshot.json), [snapshot ejemplo 1.1](examples/event-snapshot.json), [grafo de reconocimiento CHEF 1.4.0](examples/recon-graph.json) con [su esquema](recon-graph.schema.json) y [resumen de estadísticas](examples/event-statistics.json). `npm run check:docs` valida los ejemplos de los contratos registrados.

La acción local `export_chef_snapshot` recupera desde SurrealDB un snapshot validado por `runId`; el frontend lo descarga como JSON usando el contrato Snapshot existente. No crea un formato de exportación ni modifica la versión del esquema. El JSONL BBOT de origen queda almacenado aparte para preservar procedencia y linaje.

`import_bbot_jsonl` recibe por IPC el texto de un archivo `.jsonl`/`.ndjson` (máximo 2 MiB) y un objeto `ScopePolicy 1.0.0` separado. La UI también limita el archivo de política a 64 KiB. Rust valida ambos antes de importar, guarda únicamente los datos saneados por el parser y persiste el snapshot, grafo fuente y proyección CHEF en SurrealDB. `HTTP_RESPONSE` conserva el origen canónico, estado y método, elimina el path y descarta cuerpo, cabeceras y cualquier campo no incluido en la lista permitida; genera evidencia de endpoint observado sin efectuar una solicitud. La operación no inicia descubrimiento de red. Se conserva la estructura y versión de Snapshot y ScopePolicy; una versión de escritorio anterior que no incluya el comando simplemente no ofrece importación local de JSONL.

`ScopePolicy 1.0.0` acepta `allowedAddresses` y `excludedAddresses` con IP literales o redes CIDR. Las redes se normalizan al identificador de red antes del digest; para cada servicio importado se evalúa la IP observada, y cualquier exclusión CIDR prevalece sobre una autorización coincidente. El [ejemplo CIDR](examples/scope-cidr.json) ilustra una red permitida y una subred excluida. Esto filtra datos de una importación local; no concede permiso para generar tráfico. Los archivos previos con IP literales mantienen su comportamiento y su digest.

La proyección `recon-graph 1.2.0` canonicaliza URLs HTTP(S) de activos: host/esquema en forma canónica, puerto por defecto normalizado y query/fragmento retirados según la política local de datos. Para `HTTP_RESPONSE` también se elimina el path y se usa el origen como endpoint observado. La evidencia conserva su evento fuente saneado. La clave determinista del activo puede cambiar al reimportar URLs que antes diferían solo en mayúsculas del host o puerto por defecto; los grafos ya guardados no se reescriben automáticamente.

Para eventos `OPEN_TCP_PORT` y `OPEN_UDP_PORT`, CHEF valida que `data` sea un netloc `host:puerto`, completa `host` o `port` si el export omitió uno, y rechaza valores explícitos que contradigan el netloc. El netloc solo se analiza localmente; no se resuelve el host. Exportaciones que omitan `host`/`port` siguen siendo aceptables si `data` incluye ambos; las que contienen campos inconsistentes se rechazan para no asociar evidencia a un servicio distinto.

El evento interno Tauri `scan-log` usa [scan-log.schema.json](scan-log.schema.json) y el [ejemplo 1.0.0](examples/scan-log.json). Solo describe la importación offline local: fases de inicio, evento, validación, guardado, fin o error, número de secuencia, tipo de evento y fecha observada. El mensaje visible es texto fijo CHEF; no transporta host, URL ni datos crudos de evidencia. El frontend valida el esquema antes de aceptar el evento y xterm.js solo recibe texto renderizable. Esta primera versión emite un evento por registro del fixture; los lotes y límites de frecuencia se medirán antes de habilitar otros flujos. Una ruptura incompatible requiere nueva versión del contrato interno.

La configuración local se carga mediante `load_chef_config` desde `config.toml` en el directorio de configuración de la aplicación que entrega Tauri. Si no existe, CHEF usa los valores por defecto del dominio; no crea archivos ni copia los valores por defecto de BBOT. El [DTO de configuración](chef-config.schema.json) y [ejemplo](examples/chef-config.json) exponen solo preferencias validadas, sin credenciales. `active_discovery` debe permanecer `false`; un archivo importado no habilita reconocimiento de red. Los campos wire se consideran `1.0.0`; una modificación incompatible requiere nuevo esquema, ejemplo, prueba y nota de compatibilidad. La suite del contrato se deja para la fase posterior indicada por el propietario.

El comando local `parse_chef_target_seed` devuelve target-seed `1.2.0` y conserva lectura de `1.0.0`/`1.1.0`. Añade `ORG:` y `ORG_STUB:` como semillas de organización con `host: null` y `port: null`; el valor es una etiqueta introducida por la persona usuaria, no prueba propiedad ni crea scope. La versión `1.1.0` sigue reservada a `AS`/`ASN` normalizados con `host: null`; no determina que el ASN esté asignado ni expande o consulta sus redes. Véanse los [ejemplos URL `1.0.0`](examples/target-seed.json), [ASN `1.1.0`](examples/target-seed-asn.json) y [organización `1.2.0`](examples/target-seed-organization.json). Las pruebas funcionales de conformidad se dejan para la fase posterior solicitada; el verificador documental valida los ejemplos contra el esquema.

El cambio `1.1.0` añade la fuente `bbot-ndjson` y su lector offline; `1.0.0` conserva exactamente la semántica Nmap anterior. Los lectores anteriores rechazarán `1.1.0`; los lectores CHEF actuales aceptan ambas versiones y comprueban que la fuente corresponda a la versión. Cambios aditivos posteriores usan versión minor con lector compatible; remover/renombrar campos, alterar identidad o semántica requiere versión mayor. Todo cambio wire incluye esquema, types, fixtures, prueba y nota de compatibilidad.

El parser de eventos usa `parserVersion 1.1.0`: aplica el valor predeterminado `scope_distance = 10` cuando el campo falta y exige un entero no nulo cuando aparece. Un cero explícito se conserva como semilla raíz. El Snapshot sigue en `1.1.0`; la versión del parser distingue la semántica. El esquema admite parser `1.0.0` y `1.1.0`, mientras la validación de aplicación exige que la fuente y versión del Snapshot coincidan. Como el `runId` incluye `parserVersion`, reimportar bytes iguales cambia el ID determinista de ejecuciones creadas por el parser `1.0.0`; los registros ya guardados no se reescriben.

El spike HTTP usa un envelope **propuesto 1.1.0**, [schema separado](http-observation.schema.json); no fingir que el núcleo Nmap v1 ya lo ingiere. Ese gate evita compatibilidad falsa entre contratos distintos.

El grafo interno CHEF `1.4.0` no cambia el Snapshot público `1.1.0`. Conserva el evento BBOT fuente, adapta tipos conocidos a activos/evidencia CHEF y contabiliza los aún no adaptados. `FINDING` se proyecta como candidato. `parent_domain_of`, los servicios `TCP_PORT_HINT`, y la entidad `organization` usan calificación `hint`: dominios padre se derivan de la sección ICANN de la PSL; TCP 80/443 se sugieren para dominios/IP; las URLs aportan su puerto explícito o predeterminado. `chef_org_stub_hints` deriva nombres candidatos desde dominios raíz (`scope_distance = 0`), el campo `stub` de `SOCIAL` y `tenant-names` de `AZURE_TENANT`, con orden determinista, límites y sin red. Para dominios, genera la forma Unicode desde punycode mediante IDNA y su alias ASCII transliterado con `deunicode`; ambos son activos candidatos con evidencia derivada del mismo dominio raíz. La transliteración es funcionalmente equivalente para producir un alias ASCII, aunque la tabla de caracteres puede diferir de Python `Unidecode`. Ningún nombre sugiere propiedad confirmada. Nunca se marca un puerto sugerido como abierto. `DNS_NAME_UNRESOLVED` conserva `unresolved`; los dominios/IP derivados quedan como `hint`, y una observación directa prevalece al fusionarlos. El lector `1.4.0` conserva compatibilidad con `1.0.0`–`1.3.0`; consumidores anteriores rechazan `1.4.0` hasta actualizar su esquema. Los módulos offline solo se ejecutan si se habilitan explícitamente en la configuración o el perfil de importación, y mantienen el linaje. Los registros previos sin `chef_recon_graph` siguen siendo legibles; al reimportar un JSONL se construye la proyección. `npm run check:docs` valida esquema y ejemplo; las pruebas funcionales de equivalencia y controles negativos se aplazan por indicación del propietario.

Ejemplo de activación local opcional:

```toml
[modules]
chef_url_parents = true
chef_url_attribute_hints = true
chef_ip_range_expansion = true
chef_dns_parent_hints = true
chef_common_port_hints = true
chef_org_stub_hints = true
```

Una configuración ausente o sin estas claves no ejecuta derivaciones; `active_discovery` debe permanecer deshabilitado. La clave `ScopePolicy` de una importación limita evidencia/proyección y no es permiso para generar tráfico.

La migración a Rust no cambia por sí sola el formato wire. El núcleo Rust debe demostrar conformidad con los mismos fixtures/golden que TypeScript antes de reemplazarlo. Los mensajes IPC de Tauri son contratos internos nuevos: deben versionarse y validar payloads en Rust y TypeScript. El esquema SurrealDB es persistencia interna, no formato de exportación. Cualquier ampliación de snapshot sigue el procedimiento de versión, esquema, ejemplo, prueba y nota de compatibilidad.
