# Roadmap hasta CSH

Presentación: **31-10-2026**. Trabajo planificado 29-09 a 30-10. Se reemplaza el supuesto de siete semanas de Specter por cuatro sprints ajustados al calendario real. Capacidad neta: [102.9 h](../research/feasibility.md), con 25 % de reserva; revisar si 10 h semanales no son por persona.

| Sprint | Fechas      | Capacidad neta / plan | Objetivo e incremento                                         | Historias           | Salida, riesgo y recorte                                                             |
| ------ | ----------- | --------------------- | ------------------------------------------------------------- | ------------------- | ------------------------------------------------------------------------------------ |
| S1     | 29-09–05-10 | 22.5 h / 21 h         | Importación pequeña con evidencia en un explorador de fixture | C01,C02,C03,W01,Q01 | Pipeline/CI + UI fixture. Riesgo setup; eliminar módulos extra si falla              |
| S2     | 06-10–12-10 | 22.5 h / 21 h         | Selección/evidencia/export; frontera API y decisión HTTP      | W02,W03,C04,I02     | Demo base completa; freeze 12-10. API se recorta a cargar snapshot local             |
| S3     | 13-10–19-10 | 22.5 h / 21 h         | Accesibilidad, cancelación real y benchmark ampliado          | W04,I03,Q02         | Controles negativos/rendimiento medidos. Recortar timeline antes que scope/evidencia |
| S4     | 20-10–30-10 | 35.4 h / 31 h         | Endurecimiento, tres ensayos y paquete de presentación        | Q03,Q04,B01,Q05     | Demo offline5 min y respaldo; Burp spike se pospone si afecta ensayo                 |

Plan total94 h; holgura neta8.9 h. La estimación inicial98 h de factibilidad es un techo de sizing; backlog refinado94 h. Nunca consumir la reserva para prometer nuevos módulos.

Gate05-10: inspeccionar horas reales, UI, parser hostile y contratos. Gate12-10: congelar scope. Gate19-10: cero bugs que corrompan evidencia o salgan de scope. Gate30-10: build reproducible y tres ensayos; si falla, mostrar snapshot validado + CLI como respaldo sin fingir funcionalidades.

Planning30 min al inicio, sincronización10 min lunes/miércoles/viernes, review30 min con demo y retro15 min al cierre; refinement15 min semanal. Ceremonias cuentan dentro de capacidad; combinar review/retro cuando ayude. Facilitación Diego S1/S3, Jhojan S2/S4, propuesta pendiente de acuerdo.

Después CSH: [roadmap Burp](burp-roadmap.md). Burp Suite Day no tiene fecha confirmada; no condiciona el deadline CSH ni demuestra invitación.
