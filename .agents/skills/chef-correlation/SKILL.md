---
name: chef-correlation
description: Diseñar o cambiar reglas CHEF de correlación activo-pasivo, deduplicación y explicación con evidencia.
---

# Correlación CHEF

Lee `docs/product/scope.md`, ADR 0002/0003, `docs/contracts/README.md` y `docs/research/recon-correlation-2026-09.md`. Distingue arquitectura propuesta de contrato aceptado.

Mantén observaciones originales, fuente, locator, digest, tiempo, scope y versión de regla. Usa claves tipadas y comparación acotada a proyecto/scope; fusiona solo identidad fuerte definida en contrato. Hostname, IP compartida, banner, certificado o coincidencia temporal son candidatos explicables, no confirmación de propiedad o vulnerabilidad. Conserva contradicciones y controles negativos; informa duplicados/descartes con motivo. No conviertas `open|filtered` en `open`.

Antes de cambiar contrato, actualiza schema, ejemplo, pruebas y compatibilidad. Prueba idempotencia, falsos enlaces, referencias, fechas y regresión del fixture Nmap. `npm run check` es gate; mide rendimiento con corpus y hardware declarado antes de prometer tiempo real. No ejecutes reconocimiento activo sin scope autorizado.
