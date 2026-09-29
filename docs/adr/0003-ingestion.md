# ADR 0003: Ingestión y fusión

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

## Contexto

Imports no confiables y límites visibles; fallo sin publicar parcial.

## Opciones

Parser propio, parser XML mantenido, subprocess Nmap.

## Decisión

Adaptador XML con librería fijada en lock; sin DTD externo/entities/red/shell. Application aplica scope antes de correlación. Fallo atómico.

## Consecuencias

Subset UTF-8, una dirección IP por host, TCP/UDP, solo up/open. No OS, scripts, hostnames ni banners. Worker necesario para cancelación en curso responsiva.

## Revisión prevista

Gate 05-10; revisión adversarial antes de ampliar subset.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
