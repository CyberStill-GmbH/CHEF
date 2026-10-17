# CHEF web · primera entrega frontend smoke

Esta entrega es una interfaz React local contra el fixture canónico. Demuestra la carga y exploración accesible de `Snapshot 1.0.0`; no acredita todavía la plataforma web multiusuario ni el cumplimiento completo de FR-04.

## Implementado

- React, TypeScript y Vite.
- Carga directa del fixture canónico `docs/contracts/examples/snapshot.json`, sin red.
- Validación de `Snapshot 1.0.0` con AJV y el schema canónico.
- Comprobación de integridad referencial antes de entregar el snapshot a React.
- `ApiClient` desacoplado de React y `FixtureApiClient` offline.
- Estados accesibles `loading`, `empty`, `error` y `success`.
- Reintento, cancelación con `AbortController` y descarte de resultados tardíos.
- Lista semántica y accesible de assets con selección mediante botones nativos.
- `EvidencePanel` para recorrer Asset → Observation → Evidence y mostrar relaciones y paths contractuales.
- Pruebas automatizadas de contrato, estados, selección, teclado, referencias ausentes y contenido hostil.
- Seguridad básica frente a contenido hostil: interpolación de texto React, sin HTML activo ni enlaces derivados de datos importados.

## Diferido / no implementado

- API HTTP real.
- OAuth.
- Proyectos multiusuario y autorización de servidor.
- PostgreSQL/Prisma.
- Grafo/Cytoscape.
- Fuentes OSINT.
- SSE o actualizaciones en tiempo real.
- Exportación visual.
- Despliegue y configuración operativa de headers de seguridad.
- Cumplimiento completo de FR-04; faltan el grafo equivalente, evaluación integral de accesibilidad y validación de uso con personas.

## Verificación

Desde la raíz del repositorio:

```text
npm run web:check
```

El gate raíz `npm run check` incluye este comando además de conservar los checks del core, documentación y arquitectura.
