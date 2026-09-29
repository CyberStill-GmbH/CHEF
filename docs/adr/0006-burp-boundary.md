# ADR 0006: Frontera Burp

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

## Contexto

BApp tiene instalación y runtime distintos a la web. Dependencia externa obligatoria perjudica instalación.

## Opciones

A) Java/Montoya autónoma con subset útil; B) puente a app local.

## Decisión

A es camino preferido. Spike convierte selección in-scope a Observation HTTP propuesta 1.1.0. B solo opcional, manteniendo valor autónomo.

## Consecuencias

Implementación de reglas Java requiere golden fixtures para no divergir. No embeber Node/React ni abrir listener en spike. Dependencias API provided; futuras runtime incluidas en JAR. No garantizar aceptación por esta elección.

## Revisión prevista

Tras carga y unload reales en Burp; antes de submission.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
