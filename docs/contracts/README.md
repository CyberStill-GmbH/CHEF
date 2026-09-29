# Contratos

La prioridad web se discute en el [borrador de API multiusuario](web-api-proposal.md). La [API loopback](api.md) y los [eventos SSE](events.md) son diseños históricos no implementados; **no son contratos estables**. El único wire ejecutable validado hoy es Snapshot 1.0.0, más el envelope HTTP separado del spike Montoya.

`snapshot.schema.json` es la autoridad wire del incremento offline, JSON Schema 2020-12, versión 1.0.0; types del dominio deben concordar mediante pruebas. `Asset`, `Evidence`, `Observation`, `Relationship`, `ExposurePath`, `ScanRun` y `ScopePolicy` tienen campos requeridos y propiedades extra prohibidas. `Finding` está definido como contrato reservado; el motor actual no lo produce.

## Semántica

- Asset: entidad estable; firstSeen/lastSeen son tiempos observados, no reloj de importación; ≥1 Observation.
- Observation: hecho open-port, source/version/run y locator dentro del archivo; una Evidence; timestamps UTC RFC3339.
- Evidence: SHA-256 de bytes, parserVersion, importedAt y locator raíz. No incorpora raw sensible ni ruta de filesystem. Reimportar idénticos bytes/scope/versión produce los mismos IDs.
- Relationship: exactamente dos Asset existentes; ≥1 Observation existente, assertion observed.
- ExposurePath: en v1 exactamente dos assets y una relación; assertion inferred, regla open-service/v1, vulnerabilidad confirmada siempre false.
- ScanRun: snapshot publicado únicamente al completarse; IDs deterministas por bytes/scope/parser. La API futura manejará intentos cancelados/fallidos con IDs independientes.
- ScopePolicy: allowlist literal, exclusions tienen precedencia, puertos/protocolos y límites. Importar un archivo no autoriza tráfico ni modifica la policy.
- Finding reservado: necesita evidence vía Observation y revisión humana; todavía sin API/implementación. Un puerto abierto no genera Finding.

Confianza 0..1: en v1 describe fidelidad al reporte importado. No está calibrada estadísticamente. La fusión usa máximo sin interpretar duplicados como evidencia independiente. Conservar contradicciones/estado closed en futuras versiones requiere nuevo tipo de Observation.

## Identidad y fusión

SHA-256 de JSON de una lista ordenada de partes UTF-8; prefijo asset/observation/evidence/run/relationship/path. No concatenar campos ambiguos sin delimitación. Keys address/service se documentan en SOLID. Scope normaliza y ordena sus arrays antes de hashing. Golden fixture fija IDs para evitar divergencia Java/TS.

Schema verifica forma, no integridad referencial ni autoría. Pruebas del pipeline verifican referencias; importación de snapshots externos requerirá un verificador semántico antes de persistirlos. Los exporters actuales solo emiten resultados del pipeline validado.

## Ejemplos y compatibilidad

[Scope ejecutable](../../fixtures/scope/lab.json), [Nmap sintético](../../fixtures/nmap/lab.xml), [snapshot ejemplo](examples/snapshot.json). `npm run check:docs` valida todos los ejemplos.

Cambios aditivos de tipos usan versión minor con reader explícitamente compatible; remover/renombrar campos, alterar identidad o semántica usa major. Reader actual rechaza cualquier versión distinta de 1.0.0. Los cambios requieren schema, types, fixtures, prueba y ADR.

El spike HTTP usa un envelope **propuesto 1.1.0**, [schema separado](http-observation.schema.json); no fingir que el núcleo Nmap v1 ya lo ingiere. Ese gate evita compatibilidad falsa entre contratos distintos.
