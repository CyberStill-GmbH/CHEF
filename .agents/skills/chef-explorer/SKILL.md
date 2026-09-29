---
name: chef-explorer
description: Construir la exploración React de snapshots CHEF con evidencia accesible.
---

# chef-explorer

Usar al trabajar en apps/web, selección de grafo o panel de evidencia.

Leer ADR0007, FR04 y contrato de snapshot. Grafo y lista equivalente accesible; seleccionar entidad revela observaciones/hash/locator y assertion. Labels como texto plano; no navegar enlaces importados ni ejecutar HTML. No recalcular identidad/fusión en React. Medir render de1000 nodos en máquina real antes de prometer tiempo real. Conservar estados empty/error/loading y recuperación mediante snapshot validado.

Las rutas mencionadas son relativas a la raíz del repositorio CHEF. Ejecutar npm run check para los cambios pertinentes y registrar resultados reales.
