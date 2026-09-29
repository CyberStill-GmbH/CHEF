---
name: chef-safe-ingestion
description: Añadir o revisar parsers y adaptadores de evidencia no confiable en CHEF.
---

# chef-safe-ingestion

Usar al modificar imports de XML/HTTP, scope o límites del pipeline.

Leer docs/security/threat-model.md y scope.md. Adaptador entrega registros normalizados o error tipado, sin red/shell ni grafo parcial. Comprobar byte/depth/count limits antes de trabajo expansivo. Probar XML/UTF-8 malformado, DTD/entities, scope/exclusion y control negativo. Red activa exige política de conexión/redirect/DNS específica ya implementada; no inferir autorización desde un input importado.

Las rutas mencionadas son relativas a la raíz del repositorio CHEF. Ejecutar npm run check para los cambios pertinentes y registrar resultados reales.
