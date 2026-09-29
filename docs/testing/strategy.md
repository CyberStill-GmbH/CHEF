# Estrategia de calidad

Core: parser unitario con XML válido/malformado/XXE/UTF-8 inválido/puertos inválidos, scope/exclusions/IPv6 y límites. Correlación: fixture positivo y negativo, conmutatividad por orden, idempotencia de observaciones, integridad referencial y timestamps. Integration: pipeline → schema; CLI stdout JSON y fallo sin salida parcial. Contract: ejemplos schema y versiones desconocidas.

Arquitectura: AST identifica imports del dominio/application y rechaza dependencias hacia infraestructura/UI; SOLID también necesita revisión humana. Docs: enlaces locales, JSON, YAML de skills/workflow, Mermaid parse y requisitos/backlog enlazados. Build/typecheck/lint/format obligatorios; sin tests de snapshots de texto cosmético.

## Incrementos pendientes de la web

- Dataset ampliado: 100/1 000 observations, 2 revisores, confusiones etiquetadas y memoria máxima registrada.
- UI: interacción teclado, panel de evidencia, empty/error/loading, grafo con 1 000 nodos, lectores de pantalla mediante lista equivalente.
- API/DB: dos usuarios y proyectos, autorización por objeto/exportación, migración y rollback, importación transaccional e idempotente; probar fallo de fuente y cuota sin fabricar observaciones.
- Fuente pasiva: contrato, procedencia por registro, términos/permiso, frescura, respuesta vacía, error y fixture sintético etiquetado. La CLI actual no prueba correlación multifuente.
- Worker activo posterior: cancelar durante parsing y no emitir updates tras terminal; solo con nueva autorización/ADR.
- HTTP de salida posterior: DNS rebinding, redirect externo, timeouts, puerto excluido y red simulada (sin Internet). CHEF no lanza descubrimiento activo en esta fase.
- Burp: [matriz manual](../bapp/manual-tests.md), conformidad TS/Java y unload repetido.

## Definition of Done

Aceptación éxito/fallo; review de otro integrante; checks verdes; contrato/ADR actualizado si cambia; nada de datos reales/secretos; scope y error observables; incremento demostrable en el entorno declarado; limitaciones documentadas. Para la web privada, probar OAuth, aislamiento y recuperación antes de decir que está lista. Para esta inicialización automatizada los checks sirven como gate técnico, pero no se inventa revisión humana.

Presupuestos de CI: un job Linux para npm, uno JDK para spike y [CodeQL](../governance/ci-security.md) para TypeScript/Java; concurrencia cancela runs antiguos de la misma rama. No empezar con matrices de OS/versiones sin bugs de plataforma que las justifiquen. Ampliar a Windows cuando se empaquete CLI; Node se fija en 24 para evitar variación de runtime.
