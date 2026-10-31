# CHEF Web · explorador offline

La interfaz React/Vite corre en un navegador y presenta el fixture contractual sin transmitir evidencia. Incluye cuatro vistas: reconocimiento, evidencia, fuentes y ajustes; un explorador de grafo con selección/lista accesible; una consola CHEF acotada; estadísticas y exportaciones locales disponibles en el cliente. La consola no es una shell del sistema y el grafo del fixture no representa actividad de red en vivo.

## Implementado

- React, TypeScript y Vite; diseño web adaptable a escritorio y móvil.
- Carga directa del fixture canónico `docs/contracts/examples/snapshot.json`, sin red.
- Validación de `Snapshot 1.0.0` con AJV y el schema canónico.
- Comprobación de integridad referencial antes de entregar el snapshot a React.
- `ApiClient` desacoplado de React y `FixtureApiClient` offline.
- Grafo CHEF interactivo con ajuste al tamaño del navegador, lista/inspector y estados de evidencia.
- Consola xterm.js limitada a comandos CHEF documentados; no ejecuta comandos del sistema.
- Estados accesibles `loading`, `empty`, `error` y `success`.
- Reintento, cancelación con `AbortController` y descarte de resultados tardíos.
- Lista semántica y accesible de assets con selección mediante botones nativos.
- `EvidencePanel` para recorrer Asset → Observation → Evidence y mostrar relaciones y paths contractuales.
- Pruebas automatizadas de contrato, estados, selección, teclado, referencias ausentes y contenido hostil (pendientes de ejecución para este corte visual).
- Seguridad básica frente a contenido hostil: interpolación de texto React, sin HTML activo ni enlaces derivados de datos importados.

## Diferido / no implementado

- API HTTP real.
- OAuth.
- Proyectos multiusuario y autorización de servidor.
- PostgreSQL/Prisma.
- Fuentes OSINT.
- API HTTP Rust conectada a la UI, autenticación y proyectos multiusuario.
- Importación de archivos directamente desde el navegador y persistencia compartida.
- Streaming de trabajos reales; la actividad actual usa eventos de fixture.
- Despliegue y configuración operativa de headers de seguridad.
- Evaluación integral de accesibilidad y validación de uso con personas.

## Verificación

Desde la raíz del repositorio:

```text
npm run web:check
```

El gate raíz `npm run check` incluye este comando además de conservar los checks del core, documentación y arquitectura. En este corte se validó el build web y el renderizado responsive en navegador; no se ejecutaron pruebas automatizadas.
