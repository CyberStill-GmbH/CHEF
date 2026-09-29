# PRD: CHEF

**Problema:** un analista recibe Nmap, tráfico HTTP y endpoints en formatos distintos y pierde tiempo decidiendo qué entidades coinciden y por qué una relación merece atención.

**Usuario inicial:** estudiantes y analistas que trabajan en un laboratorio autorizado y deben explicar su análisis a una audiencia técnica. Usuario posterior: operador de Burp que necesita relacionar evidencias seleccionadas sin abandonar su sesión.

**Propuesta:** un mapa local donde cada relación tiene procedencia, regla e incertidumbre visibles. CHEF prioriza explicación y reproducción sobre cantidad de fuentes. La diferenciación es una hipótesis de producto, no una declaración de unicidad mundial.

Escenario CSH: importar un archivo sintético; ver cómo dos observaciones se fusionan en un servicio; inspeccionar la evidencia; mostrar que otro host con el mismo nombre no se fusiona; explicar una exposición; exportar el resultado. Escenario Burp posterior: seleccionar requests en scope, unir endpoints con evidencia de servicios, inspeccionar razones y exportar sin servidor externo obligatorio.

## Éxito falsable

- 100 % de activos y relaciones del dataset de demo con cadena de evidencia resoluble.
- Cero fusiones en controles negativos; precisión ≥ 0.98 y recall ≥ 0.95 en dataset ampliado etiquetado por dos revisores. Son objetivos propuestos, no resultados obtenidos.
- p95 de ingestión del núcleo < 500 ms para 1 000 observaciones; p95 hasta grafo web visible < 1 s. Medir por separado en máquina de demo e informar CPU/RAM/versiones.
- Exportación reproducible con mismo input, versión, scope y reloj congelado; identidad del grafo estable si cambia solo la hora de importación.
- Demo completa en 5 min en tres ensayos sin Internet; máximo un paso manual de recuperación.
- Al menos 4 de 5 evaluadores explican correctamente la evidencia y la diferencia entre exposición y vulnerabilidad en una sesión guiada. Estudio aún no realizado.

No se prometen hallazgos nuevos, exactitud en infraestructura real ni aprobación de BApp Store. El presupuesto y los recortes están en [factibilidad](../research/feasibility.md); requisitos en [trazabilidad](requirements.md).
