# Después de CSH: Burp Suite Day y BApp Store

No es trabajo completado ni submission realizada. Fecha Burp Suite Day desconocida. Elegir capacidad tras31-10; tamaño estimado40–70h netas, revisable tras spike.

## Opciones y costo

A: Java/Montoya autónoma, selección/request → evidencia/relaciones → vista/lista → export. JAR útil sin servidor/Node/Nmap obligatorio. Estimación30–45h para subset/persistencia/UI más10–25h para pruebas/review/submission. Complejidad: conformidad Java/TS, Swing, unload y proyectos grandes.

B: adaptador Java a app local opcional. Aporta visualización React avanzada, pero añade configuración, auth loopback y disponibilidad del servicio. Estimación adicional15–25h y dos runtimes que mantener. La documentación exige dependencias incluidas para instalación sencilla; un servicio obligatorio pone en riesgo ese criterio. No hay prohibición universal documentada aquí ni garantía de aceptación. Diseñar valor útil en modo autónomo y solicitar evaluación solo cuando esté demostrado.

## Gates

1. Spike: JAR compila; cargar en Burp real, selección local in-scope y export HTTP1.1.0 validado; descargar sin threads huérfanos. Registrar versión/API/JDK y capturas.
2. Conformidad: wire version compartida, fixtures de identidad/fusión y golden tests JVM/TS; importar XML local sin ejecutar Nmap; mantener paths sensibles como hash por defecto.
3. Utilidad autónoma: evidencias Nmap+HTTP dentro de Burp, explicación y export; no depender de web para la tarea principal. Comparar contra NMAP Parser, Nmap Scanner y Site Map con operadores.
4. Robustez: tráfico hostile, 10k/100k mensajes por batches, límites/memoria, EDT responsiva, background exceptions, unload repetido y offline.
5. Packaging: build limpio reproducible, versión mínima Burp comprobada, licencia de todos los autores y dependencias; README inglés, guide/capturas/data fixture, changelog y hashes.
6. Revisión del equipo y de requisitos vigentes; completar [checklist](../bapp/acceptance.md) y preparar [borrador](../bapp/submission-draft.md). La publicación en extension-portal requiere decisión posterior del equipo; este encargo no envía submission.

Dentro de Burp: selección Site Map/requests, lista de activos/endpoints, razones/provenance/export. Web: grafo completo, análisis comparativo/timeline, solo opcional. Mantener código del núcleo sin APIs Burp. IA no entra en diferenciación inicial; si se incorpora se revisan criterios vigentes y manejo de datos.
