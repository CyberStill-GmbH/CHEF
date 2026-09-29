# Borrador de frontera API para CHEF web

Fecha 29-09-2026. **Diseño para discusión; ninguna ruta existe ni es contrato publicado.** El [Snapshot 1.0.0](README.md) sigue siendo el único wire validado. Antes de implementar se aprobará OpenAPI/schema, ejemplos, prueba positiva/negativa y nota de compatibilidad. Este borrador reemplazaría la [API loopback histórica](api.md) para el producto web.

## Principios

- GitHub OAuth autentica en el servidor; CHEF mapea identidad a usuario/membresía de proyecto. Cookies de sesión protegidas y CSRF/origen según flujo elegido; el navegador no ve secretos de GitHub ni de OSINT.
- Toda ruta de proyecto verifica membresía y acción en servidor; IDs enviados por cliente jamás son prueba de permiso. Listado, detalle, import, revisión y exportación se prueban con dos usuarios/proyectos (CWE-862/639).
- DTO de lectura representa entidad/observación/relación/explicación, no modelos Prisma. Cada observación incluye fuente, fecha, locator, digest y estado; cada enlace incluye regla/version/evidencias. `Snapshot 1.0.0` conserva su significado; multifuente necesita versión nueva, no reinterpretación.
- Upload XML con tamaño/tipo/scope acotados, fallo atómico y run auditado. La primera implementación puede responder al terminar importación pequeña; un job/202/cancelación solo cuando worker real lo soporte.
- No aceptar URLs arbitrarias, targets de scan ni proveedores elegidos libremente por el cliente. Cada conector habilitado tiene configuración/permiso y cuota del proyecto. Un archivo importado no amplía scope.

## Recursos candidatos para refinement

| Operación prevista                             | Uso y comprobación principal                  | Error/estado que debe distinguirse            |
| ---------------------------------------------- | --------------------------------------------- | --------------------------------------------- |
| `GET /v1/me`                                   | identidad y proyectos accesibles              | sesión ausente/expirada                       |
| `POST /v1/projects`, `GET /v1/projects`        | crear/listar proyectos del usuario            | nombre inválido, sin acceso                   |
| `POST /v1/projects/{id}/imports/nmap`          | multipart XML + policy explícita; invoca core | tamaño, XML hostile, fuera de scope, rollback |
| `GET /v1/projects/{id}/runs`                   | estado, origen, fecha, conteos de exclusión   | proveedor/DB falló ≠ cero hallazgos           |
| `GET /v1/projects/{id}/assets`                 | paginación/filtrado por tipo/estado/frescura  | cursor/revisión inválidos                     |
| `GET /v1/projects/{id}/relationships/{rid}`    | endpoints, regla/version, evidencia/fechas    | candidato/conflicto nunca confirmado          |
| `POST /v1/projects/{id}/sources/{sid}/refresh` | solo si fuente/permiso y worker aprobados     | cuota, permiso, error, stale                  |
| `GET /v1/projects/{id}/exports/{eid}`          | snapshot versionado o imagen segura           | usuario ajeno/artefacto vencido               |

No se ha elegido forma final de paginación, session store, error envelope, import síncrono/asíncrono ni retención. Esos puntos son **decisiones a probar**, no huecos que el frontend pueda inventar. Se pactará fixture/DTO entre César, Diego y Jhojan antes de desarrollar UI contra una ruta. El esquema de export visual tendrá evaluación de SVG/XSS; JSON conserva semántica de evidencia y manifest de filtros.

## Compatibilidad y migración

El cliente nuevo se versiona `/v1` cuando exista implementación y contrato formal; no hay clientes del diseño histórico que migrar. La CLI y `Snapshot 1.0.0` continúan funcionando. Si las observaciones pasivas no caben en ese snapshot, publicar nuevo schema/ejemplo/reader/version y documentar cómo convive con v1; los readers actuales rechazan versiones desconocidas. Un servicio desplegado no afirma eventos SSE ni “tiempo real” sin latencia/estado de fuente medidos.
