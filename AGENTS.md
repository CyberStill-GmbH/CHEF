# Colaboración en CHEF

- Inspecciona `git status` antes de modificar; conserva trabajo ajeno. El nombre vigente es CHEF.
- Lee `docs/product/scope.md` y el ADR relevante. Mantén visible la diferencia entre implementado, propuesto y validado manualmente.
- Dominio sin filesystem, red, frameworks ni Montoya. Application depende de puertos; adapta infraestructura en `packages/adapters` y compón en apps.
- Preserva IDs deterministas, evidencia y control negativo. No conviertas inferencias en vulnerabilidades confirmadas.
- No ejecutes descubrimiento activo sin scope autorizado. Los fixtures y la demo predeterminada son offline.
- Cambios de contrato requieren esquema, ejemplo, prueba y nota de compatibilidad. No dupliques reglas en Java: usa fixtures de conformidad compartidos.
- Ejecuta `npm run check`; para cambios Montoya compila y registra por separado pruebas manuales realizadas y pendientes.
- PR pequeño, criterio Given/When/Then y revisión por otra persona en desarrollo del equipo. Esta inicialización automatizada no sustituye esa revisión humana.
- Guías locales en `.agents/skills`; no instales guías globales ni añadas dependencias sin propósito.
