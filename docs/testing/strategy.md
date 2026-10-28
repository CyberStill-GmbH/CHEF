# Estrategia de calidad

Core: pruebas unitarias de XML válido o malformado, XXE, UTF-8 y puertos inválidos, alcance/exclusiones, IPv6 y límites. Correlación: fixture positivo y negativo, independencia del orden, idempotencia de observaciones, integridad referencial y marcas de tiempo. Integración: pipeline → esquema; salida JSON de CLI y fallo sin publicar resultados parciales. Contratos: ejemplos de esquema y rechazo de versiones desconocidas.

Arquitectura: AST identifica imports del dominio y la aplicación, y rechaza dependencias hacia infraestructura o interfaz; SOLID también requiere revisión humana. Documentación: enlaces locales, JSON, YAML de skills y workflow, diagramas Mermaid y referencias de requisitos/historias. Compilación, tipos, lint y formato son obligatorios; no se añaden pruebas de snapshots de texto cosmético.

## Validaciones previstas para escritorio

- Core Rust: paridad de IDs y `Snapshot 1.0.0` para Nmap; para `1.1.0`, importador JSONL limita tipos a puertos abiertos, respeta scope y conserva procedencia. Fallos cerrados para XML/JSONL/scope inválido, control negativo y salida reproducible con reloj fijo.
- SurrealDB embebida/RocksDB: reapertura desde disco, versiones de esquema desconocidas, escritura atómica, concurrencia, compatibilidad de esquema, backup/restauración y cierre inesperado. El adaptador inicial guarda el snapshot completo; aún no prueba el grafo normalizado.
- Tauri/IPC: validar payloads, límites, errores, cancelación y secuencia de eventos con fixtures offline antes de integrar comandos o listeners reales.
- Interfaz: navegación por teclado, evidencia y estados vacío/error/carga; cuando se integre xterm.js, comprobar que los logs hostiles se representen como texto y que la suscripción se pueda detener.
- Adaptadores externos y descubrimiento activo: fuera del corte actual. Requieren decisión, autorización y pruebas aisladas; los checks offline no se presentan como prueba de red.
- Burp: [matriz manual](../bapp/manual-tests.md), conformidad TS/Java y descarga repetida de la extensión en su fase posterior.

## Definición de terminado

Criterios de aceptación positivos y negativos; revisión de otro integrante; verificaciones automáticas satisfactorias; contrato/ADR actualizado si cambia; sin datos reales ni secretos; alcance y errores observables; incremento demostrable en el entorno declarado; limitaciones documentadas. Para la web privada, probar OAuth, aislamiento y recuperación antes de declararla lista. En esta inicialización automatizada, las verificaciones son una puerta técnica; no sustituyen una revisión humana.

Presupuestos de CI: un job Linux para npm, uno JDK para spike y [CodeQL](../governance/ci-security.md) para TypeScript/Java; concurrencia cancela runs antiguos de la misma rama. No empezar con matrices de OS/versiones sin bugs de plataforma que las justifiquen. Ampliar a Windows cuando se empaquete CLI; Node se fija en 24 para evitar variación de runtime.
