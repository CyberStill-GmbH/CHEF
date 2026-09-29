# Verificación del incremento

Verificación local ejecutada el **29-09-2026**, antes de publicar el incremento en `docs`. La integración continua repite las comprobaciones en Linux; su resultado definitivo queda asociado al commit y al pull request en GitHub.

## Entorno y resultados ejecutados

Windows, Node.js 24.16.0, AMD Ryzen 5 8645HS y 7.26 GiB de memoria física reportada por el sistema. Para Java se utilizaron Temurin 21.0.12.1+1 y Maven 3.9.16, descargados de distribuidores oficiales y verificados mediante sus checksums; permanecen fuera de Git.

- `npm run check`: correcto. ESLint, TypeScript estricto, **11 pruebas**, contratos, documentación, fronteras de arquitectura y formato.
- Documentación: 54 archivos Markdown, enlaces internos, dos diagramas Mermaid, seis skills locales, JSON/YAML, registro de fuentes y correspondencia entre requisitos e historias validados.
- Arquitectura: cinco módulos del dominio/aplicación comprobados mediante AST para evitar dependencias de infraestructura y operaciones de red. Esto complementa la revisión de SOLID; no demuestra automáticamente todos sus principios.
- `npm audit --audit-level=high`: **0 vulnerabilidades reportadas** en el lockfile evaluado. Es una comprobación puntual, no una garantía futura.
- `mvn -B -ntp -f extensions/burp/pom.xml clean verify`: correcto, **5 pruebas Java**. El fixture HTTP compartido coincide byte a byte con TypeScript.
- Dos builds limpios producen el mismo JAR. SHA-256: `5dba24b388193b4c74851083d167a778dba8b4c1d879f37435cd0024be23ff29`. La inspección del JAR confirma que contiene únicamente clases de CHEF y metadatos Maven; Montoya se resuelve como dependencia proporcionada por Burp.

## Benchmark ejecutado

`npm run benchmark`, dataset `synthetic-lab/v1`: 1 163 bytes, diez calentamientos y cien muestras. Latencia local p50 **0.3814 ms**, p95 **0.6541 ms**. Tres observaciones se correlacionan en dos servicios, con trazabilidad completa y repetibilidad usando reloj fijo. En sus etiquetas: un verdadero positivo y cero falsos positivos; precisión y recall 1.0.

El conjunto contiene un único par positivo y dos negativos. Estos valores describen exclusivamente el fixture: no estiman precisión en redes reales, capacidad de la UI ni rendimiento con proyectos grandes. El benchmark ampliado continúa en el backlog.

## Comprobaciones pendientes

El JAR **no se ha cargado dentro de Burp**. Permanecen pendientes la matriz manual MB01–MB10, compatibilidad con la versión de Burp elegida, responsividad/unload, el explorador React, API/worker/persistencia, correlación entre fuentes y revisión humana del equipo. Véanse [pruebas BApp](../bapp/manual-tests.md), [backlog](../plan/backlog.md) y [criterios de aceptación](../bapp/acceptance.md).

El incremento establece una base ejecutable y mantenible. No se declara lista una BApp ni se afirma aceptación de PortSwigger.
