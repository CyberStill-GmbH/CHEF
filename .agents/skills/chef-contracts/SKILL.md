---
name: chef-contracts
description: Modificar contratos, identidad o reglas de fusión del grafo CHEF.
---

# chef-contracts

Usar al cambiar un campo wire, un tipo de entidad o una regla de identidad/correlación.

Leer docs/contracts/README.md y ADR0002/0003. Actualizar schema y tipos juntos. Preservar Observation→Evidence y controles de falsos positivos. Elegir major/minor según compatibilidad explícita; readers actuales rechazan versiones desconocidas. Agregar ejemplo/golden y pruebas de referencias, deduplicación y orden. Validar TS y Java por fixtures compartidos; no asumir que HTTP1.1 entra en el pipeline Nmap1.0.

Las rutas mencionadas son relativas a la raíz del repositorio CHEF. Ejecutar npm run check para los cambios pertinentes y registrar resultados reales.
