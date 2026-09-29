# Panorama y diferenciación

Consulta: 29-09-2026. Fuentes primarias en [sources.json](sources.json). Se compararon documentos y repositorios; no se ejecutaron las herramientas ni se midió su calidad. Una función no mencionada se marca como no verificada, no como ausente.

| Alternativa                                                                        | Alcance y datos/correlación                                              | Visualización/importación                                           | Despliegue, mantenimiento y licencia                                           | Implicación CHEF                                                 |
| ---------------------------------------------------------------------------------- | ------------------------------------------------------------------------ | ------------------------------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------- |
| [Burp](https://portswigger.net/burp/documentation/scanner/crawling)                | Navegación web; Scanner modela ubicaciones/enlaces como grafo            | Site Map y crawl; Nmap directo no verificado aquí                   | Desktop; documentación vigente; EULA propia, Scanner Pro                       | Grafo de navegación ya existe; priorizar evidencia entre fuentes |
| [Nmap](https://nmap.org/book/output-formats-xml-output.html)                       | Descubrimiento de hosts/servicios; XML estructurado                      | XML recomendado para procesamiento; Zenmap es otro componente       | CLI/desktop; DTD oficial; NPSL, componentes con condiciones propias            | Importar datos sin empaquetar ni ejecutar Nmap                   |
| [NMAP Parser](https://portswigger.net/bappstore/0780c0a9f12e47848a94ac3e43dccbd9)  | Importa Nmap y añade puertos web a scope                                 | GUI de importación; grafo explicativo no verificado                 | BApp; ficha 1.1 de 2017; términos del repo deben revisarse antes de reutilizar | Nunca vender un parser solo como innovación                      |
| [Nmap Scanner](https://portswigger.net/bappstore/9710f65c31044b3794df0767f8f4bd5d) | Inicia scans desde Burp                                                  | Tab, salida en vivo, parsing/export XML                             | BApp 1.0, ficha 2024; licencia de código no validada                           | Diferenciar mediante correlación, no wrapper de Nmap             |
| [Amass](https://github.com/owasp-amass/amass)                                      | Mapping de superficie y descubrimiento externo con fuentes OSINT/activas | Modelo de activos; capacidades precisas por versión requieren spike | Go; repositorio con desarrollo; Apache-2.0 y subcomponentes propios            | No competir por cobertura de reconocimiento                      |
| [SpiderFoot](https://github.com/smicallef/spiderfoot)                              | Módulos OSINT y reglas de correlación                                    | UI/CLI, visualizaciones, CSV/JSON/GEXF; puede llamar Nmap           | Python/SQLite/Docker; recomienda release estable; MIT                          | Correlación y exportación ya son comunes                         |
| [Uncover](https://github.com/projectdiscovery/uncover)                             | Hosts expuestos mediante APIs de buscadores                              | IP/port/host, JSONL; grafo no verificado                            | Go; API keys, mantenimiento en repo; MIT                                       | CHEF prioriza datos locales y operación offline                  |

## Hueco específico que debe probarse

Hipótesis: dentro de Burp, un operador obtiene valor de una cadena inspeccionable `dirección → servicio observado → endpoint seleccionado`, respaldada por múltiples evidencias y una explicación de reglas/confianza; puede exportarla y reproducirla sin servicio externo. Comparar contra Nmap sin correlacionar y contra Site Map para la misma tarea. No afirmar que ninguna otra extensión ofrece esto sin ampliar la revisión del catálogo y probar las más cercanas.

La revisión halló importadores y wrappers de Nmap. Buscar por nombre CHEF, evidence, correlation, attack surface, site map y graph antes de cada submission; registrar resultados positivos y limitaciones de búsqueda. La ausencia de la palabra graph en una lista del catálogo no prueba unicidad.

## Riesgos y respuesta

- Duplicación: entrevista a 5 operadores y prueba de tarea antes de desarrollar la BApp completa.
- Grafo con demasiado ruido: relaciones tipadas, filtros, lista alternativa y controles negativos.
- Falsa confianza: `confidence=1` en el MVP significa fidelidad al hecho del archivo, no veracidad de red ni probabilidad de vulnerabilidad.
- Integración frágil: contrato versionado y fixtures de conformidad entre lenguajes.
- Mantenimiento: revisar versiones/licencias reales del lockfile; la antigüedad de una ficha no demuestra abandono.

La originalidad es un criterio de PortSwigger; el proyecto debe presentar evidencia de utilidad, no un claim publicitario de innovación.
