# Alcance

## Must para 31-10-2026

Importación Nmap XML offline, allowlist literal, normalización de puertos y direcciones, procedencia verificable, deduplicación conservadora, grafo exploratorio React/TypeScript, panel de evidencia, exposición explicada, exportación versionada y demo recuperable. UI accesible también mediante una lista de entidades y relaciones.

El incremento actual implementa el pipeline offline y JSON; la UI, API y persistencia siguen planificadas. El spike Montoya sirve para evaluar la frontera técnica, sin desplazar el trabajo CSH.

## Should, sujetos al gate del 05-10

Importador HTTP local normalizado y correlación dirección → servicio → endpoint; filtros/timeline; persistencia local mínima. Añadir un solo módulo si parser, UI de fixture y CI funcionan y el trabajo restante cabe en capacidad.

## Later

DNS y descubrimiento activo acotado, actualización web por SSE, importación de Site Map por lotes, diffs entre ejecuciones, BApp autónoma, proyectos grandes y sincronización opcional con app local. Cinco módulos es un techo de evolución, no una promesa para octubre.

## Exclusiones

Sin explotación automática, brute force, C2, evasión, inyección, shell ni ofuscación. Sin nube/telemetría/IA obligatoria. Sin correlacionar propiedad de hosts por banners o IP compartida. No ampliar scope a partir de un archivo importado. La app no declara vulnerabilidad confirmada por un servicio abierto.

Congelar funciones CSH el 12-10. Si hay retrasos, retirar módulos adicionales y conservar evidencia, scope, exportación y demostración. Los cambios de alcance se anotan en el backlog y PRD.
