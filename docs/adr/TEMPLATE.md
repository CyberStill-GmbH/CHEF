# ADR NNNN: Título que describa la decisión

Estado: propuesto. Fecha: AAAA-MM-DD. Responsables: nombres/roles. Supersede: ADR anterior o “ninguno”.

## Contexto y decisión que se debe tomar

Problema observable, estado implementado hoy, alcance autorizado, usuarios afectados, restricciones de tiempo/equipo y requisito que motiva el cambio. Enlaces a C4, contrato y fuente primaria fechada.

## Impulsores y opciones

Qué importa para decidir y cómo se medirá: exactitud, seguridad, mantenibilidad, experiencia, coste, operación y reversibilidad. Comparar mantener estado actual con al menos una alternativa factible; no usar números de puntuación sin método o datos.

## Decisión propuesta y justificación

Elegir una opción y explicar relación entre evidencia y criterio. Distinguir implementación actual de destino. Señalar condiciones que podrían cambiar la elección.

## Consecuencias y migración

Ventajas, costes/deuda, riesgos, límites de seguridad, impacto en contratos, datos, despliegue y fases de migración/rollback. Qué trabajo queda fuera.

## Verificación

Given/When/Then positivo y negativo, fixture o entorno autorizado, métricas y responsable de revisión. Enlazar pruebas/PR cuando existan; no marcar pendientes como validados.

## Fuentes y revisión

URLs primarias con fecha de consulta en `docs/research/sources.json`; personas que revisaron y fecha de aceptación o rechazo. Si sustituye otra decisión, actualizar el índice y ambos ADRs.
