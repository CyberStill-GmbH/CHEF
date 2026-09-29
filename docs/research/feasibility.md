# Factibilidad y experimentos

**Datos aportados:** presentación 31-10-2026 y 10 h semanales. **Supuesto:** 10 h por integrante. Inicio de planificación: 29-09; implementación/evidencia hasta 30-10, 32 días ≈ 4.57 semanas. Tres personas dan ≈ 137.1 h brutas; reserva 25 % = 34.3 h; capacidad planificable ≈ **102.9 h**. Si las 10 h son del equipo completo, la capacidad baja a ≈ 34.3 h netas y debe recortarse a UI mínima + pipeline offline.

Distribución propuesta: core/fixtures 21 h, web/contratos 34 h, calidad/demo 25 h, integración 12 h, spike Burp 6 h: 98 h; holgura ≈ 4.9 h. Estimaciones de trabajo restante se recalibran tras revisar el incremento automatizado; no representan horas reales trabajadas por el equipo.

## Viabilidad

Un grafo que crece por lotes con 1 000 entidades y evidencia acotada es una meta razonable para el calendario; no se ha medido la UI. Evitar layout completo en cada evento. Empezar con importación completa y luego batches de 100 con 100–250 ms de coalescing. Un grafo ilimitado en tiempo real no es compromiso viable con estas horas.

Una BApp Java autónoma es técnicamente plausible para selección HTTP, reglas pequeñas y exportación. React no se convierte automáticamente en Swing ni TypeScript en JVM. Plan posterior necesita entre 40–70 h estimadas para persistencia, conformidad de reglas, pruebas reales de Burp, packaging y revisión. No fijar fecha de submission antes de superar gates.

Costos operativos previstos: núcleo y demo sin APIs pagas ni hosting. GitHub Actions público puede usar runners estándar sujeto a políticas de GitHub. Burp Community para pruebas manuales de selección; Pro solo para funcionalidades exclusivas. No se ha comprado licencia ni verificado costo monetario. Dependencias y hardware ya instalados reducen setup, pero falta validar Burp y JDK adecuado.

## Experimentos de primera semana

1. César/Jhojan, 4 h: 100 fixtures sintéticos; parser hostile y límite 2 MiB. Gate: sin red/XXE, sin grafo parcial; registrar memoria y tiempo.
2. Diego, 5 h: Cytoscape en React con 1 000 nodos/2 000 aristas y lista accesible. Gate: interacción útil y p95 hasta render < 1 s en máquina de demo; medir, no suponer.
3. César/Diego, 2 h: contrato de snapshot en UI y golden exports. Gate: parser/core independientes de UI.
4. César con Jhojan, 3 h: cargar spike Montoya, seleccionar una request local, exportar Observation y descargar extensión. Gate: compilación no sustituye ejecución en Burp.
5. Los tres, 2 h: ensayo de 5 min con input repetido/control negativo. Gate: evidencia comprensible para un evaluador externo.

Salida el 05-10: permitir un módulo HTTP extra solo si las cinco incertidumbres están resueltas y caben ≥ 12 h de integración/pruebas. Si falla, conservar importación única y demo explicable. Riesgos principales: exámenes, Java nuevo, rendimiento de grafo, uso incorrecto de inferencias y pérdida de datos sensibles.
