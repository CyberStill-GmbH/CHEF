# Preparación BApp Store

Consulta oficial29-09-2026, página actualizada22-09-2026: [acceptance criteria](https://portswigger.net/burp/documentation/desktop/extend-burp/extensions/creating/bapp-store-acceptance-criteria). Esta checklist resume criterios; los estados CHEF son evaluación propia y no aprobación de PortSwigger.

| Criterio oficial              | Evidencia necesaria en CHEF                                    | Estado               |
| ----------------------------- | -------------------------------------------------------------- | -------------------- |
| Función única                 | Comparativa actual y prueba de tarea de correlación explicable | Hipótesis pendiente  |
| Nombre descriptivo            | CHEF: Evidence-backed Attack Surface Correlation               | Propuesto            |
| Operación segura              | Tests hostile XML/HTTP + revisión de datos/UI                  | Core parcial         |
| Dependencias incluidas        | JAR autónomo útil, instalación limpia                          | Spike parcial        |
| Threads responsivos           | Trabajo pesado fuera EDT/handlers, excepciones registradas     | Manual pendiente     |
| Descarga limpia               | Unload libera recursos y termina workers                       | Manual pendiente     |
| Networking de Burp            | Usar Http.issueHttpRequest si se añade red                     | Spike sin red        |
| Offline                       | Demo/autonomía sin consultas online                            | Core; Burp pendiente |
| Proyectos grandes             | Batches, límites, evitar retener objetos de tráfico            | Pendiente            |
| GUI con parent                | Popups hijos del frame Burp                                    | Spike sin popups     |
| Artefacto Montoya             | Dependencia declarada en Maven/Gradle                          | Spike build          |
| IA con Burp AI predeterminado | Revisar política si se añade IA                                | No aplica hoy        |

## Gates propios adicionales

No prometer que el acrónimo CHEF describe la función por sí mismo; acompañarlo siempre del subtítulo. No publicar como BApp terminada un JAR que solo produce metadata. Evidencia mínima: tarea útil dentro de Burp, compatibilidad probada en versiones listadas, resultados manuales, README inglés, licencia/consentimiento de autores y benchmark con límites claros.

Proceso vigente: repositorio fuente, nombre/description y guía de funcionamiento/setup; después New extension submission issue en [extension-portal](https://github.com/PortSwigger/extension-portal). Ver [documentación de submission](https://portswigger.net/burp/documentation/desktop/extend-burp/extensions/creating/bapp-store-submitting-extensions). No se usará un correo antiguo como procedimiento supuesto y no se ha enviado una solicitud.
