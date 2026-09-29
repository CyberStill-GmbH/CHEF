# ADR 0001: Núcleo TypeScript

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

## Contexto

Necesitamos entregar con React/TypeScript y 102.9 h netas; un segundo lenguaje en el core aumenta setup. Burp exige frontera Java.

## Opciones

TypeScript, Go y Rust. Escala 1..5, estimación (no benchmark): aprendizaje TS5/Go3/Rust2; entrega5/4/2; web5/3/2; distribución3/5/5; seguridad de memoria4/4/5; integración JVM2/2/2. Pesos 25/25/20/10/10/10: TS4.4, Go3.45, Rust2.6.

## Decisión

TypeScript estricto en dominio/application; Node24 en CLI. Java únicamente en spike Montoya.

## Consecuencias

Comparte fixtures y tipos con futura web; no implica compartir runtime con JVM. Requiere worker/limites para operaciones pesadas. Revisar Go si descubrimiento/red supera capacidad Node; Rust solo con evidencia de necesidad.

## Revisión prevista

Gate 05-10 y antes de red activa.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
