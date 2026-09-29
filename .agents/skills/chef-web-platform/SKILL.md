---
name: chef-web-platform
description: Diseñar o implementar API, OAuth, proyectos y PostgreSQL/Prisma de CHEF web multiusuario.
---

# Plataforma web CHEF

Lee `docs/architecture/proposed-web.md`, los ADRs 0001/0004/0005 y `docs/security/threat-model.md`. Trata la arquitectura propuesta como pendiente de aceptación. El núcleo y la CLI actuales deben conservar semántica y pruebas.

El dominio no depende de HTTP, filesystem, React ni Prisma. Define puertos en application, adapta Prisma en `packages/adapters` y compón en apps. El snapshot versionado es contrato de intercambio, no modelo de tablas. GitHub OAuth autentica; CHEF autoriza por membresía de proyecto en cada operación, incluida exportación. Prueba cruces entre dos usuarios/proyectos e IDOR; protege sesiones y secretos. RLS puede reforzar, pero no sustituye, autorización del servidor.

No conectes navegador a DB ni expongas tokens de fuente OSINT. Un importador limita tamaño, formato y scope, falla atómicamente y registra provenance. Para despliegue, registrar entorno probado, migraciones/rollback, backup/restore, errores recuperables y datos no sensibles en logs. No declarar multiusuario o tiempo real hasta pruebas en entorno desplegado. Ejecuta `npm run check` y revisión humana de PR.
