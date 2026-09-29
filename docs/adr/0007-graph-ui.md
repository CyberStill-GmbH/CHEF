# ADR 0007: Librería de visualización

Estado: aceptado para el incremento inicial; decisiones futuras indicadas expresamente. Fecha: 29-09-2026.

## Contexto

Grafo con selección, filtros, evidencia y alternativa accesible; objetivo de 1 000 nodos.

## Opciones

Cytoscape.js (layouts/consulta/grafos tipados), Sigma.js (WebGL con Graphology), render SVG propio (coste de interacción/layout). Fuentes CYTOSCAPE y SIGMA.

## Decisión

Cytoscape para primer spike React. Instancia imperative encapsulada; datos/contratos separados de view. Selección paralela en lista accesible.

## Consecuencias

Preferencia por capacidades documentadas, no medición de rendimiento. Sigma alternativa si la escala GPU compensa complejidad; SVG propio solo grafo estático pequeño.

## Revisión prevista

Gate 05-10 usando máquina real; revertir elección si falla presupuesto.

Referencias: [arquitectura](../architecture/overview.md), [fuentes](../research/sources.json).
