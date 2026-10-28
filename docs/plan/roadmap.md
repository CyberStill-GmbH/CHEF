# Hoja de ruta de CHEF

Actualizado 18-10-2026. **Dirección vigente: aplicación local Tauri/Rust, con UI React y SurrealDB embebida sobre RocksDB.** El [diseño de persistencia](../architecture/persistencia-local.md), [arquitectura de escritorio](../architecture/desktop-tauri.md), [ADR 0012](../adr/0012-escritorio-tauri-rust.md) y [backlog de migración](backlog-desktop.md) contienen decisión y estados. La antigua ruta web Node/Go se conserva como antecedente, sin implementación.

## Etapa 0 · núcleo de referencia implementado

CLI y dominio TypeScript offline, importador Nmap XML acotado, alcance literal, IDs deterministas, evidencia, controles negativos y `Snapshot 1.0.0`; explorador React/Vite local y spike Java/Montoya separado. Esta etapa se mantiene disponible hasta demostrar conformidad del port Rust. No se encuentra backend Python en el checkout actual.

## Etapa 1 · vertical de escritorio offline

1. **Toolchain y shell:** Rust/Tauri v2 compilable en el sistema objetivo, capabilities mínimas y UI React actual dentro del WebView.
2. **Núcleo Rust:** portar por incrementos el parser/casos de uso necesarios; comparar salida e IDs con fixtures/golden TypeScript antes de cambiar la autoridad.
3. **Trabajo y logs:** comando asíncrono, cancelación, cola acotada y evento `scan-log` versionado; xterm.js muestra texto estructurado de una simulación offline.
4. **Persistencia local:** adaptador SurrealDB/RocksDB inicial escrito para snapshot por `runId`; validar build y completar compatibilidad de esquema, grafo, backup y restauración. Vectores quedan propuestos hasta medir un caso de uso.
5. **Entrega de octubre:** demo offline repetible y estado de cada pieza; validación manual del build en Windows. Otros sistemas se reportan pendientes hasta disponer de CI/entorno.

El [plan Scrum vigente](scrum-actual.md) divide esta etapa entre César, Diego y Jhojan. La carga Nmap, la simulación y la base local se implementan sin transmitir tráfico a objetivos.

## Etapa 2 · fuentes pasivas y sincronización optativa

Evaluar conectores individualmente tras verificar términos, cuenta, permiso, cuota y datos permitidos. Los secretos se guardan fuera de TOML; la UI enseña proveedor, datos enviados y destino antes de cada consulta. La sincronización DefectDojo es una acción separada con vista previa/confirmación, redacción, manejo de errores y registro local. No se consulta ningún dominio de cliente por defecto.

## Etapa 3 · descubrimiento activo (gate nuevo)

DNS/HTTP no se ejecuta hasta que un ADR específico documente autorización escrita, targets/IP/puertos, política de resolución/redirect, rate/concurrency, timeout, cancelación, auditoría y entorno de prueba. Sin esa autorización, la función sigue desactivada; la demo usa fixtures y se identifica como simulación.

## Etapa 4 · integración Burp

Reevaluar la utilidad del spike Montoya después de validar el escritorio local. Mantener conformidad mediante fixtures compartidos; ejecutar tests manuales de carga/unload en Burp y documentar por separado lo hecho y lo pendiente. No afirmar aceptación BApp por compilar un JAR.

Los gates cambian con evidencia y capacidad real. No confundir seleccionado, propuesto, implementado, probado automáticamente, validado manualmente ni publicado. Los registros de septiembre permanecen en las propuestas históricas; la dirección actual se enlaza desde el [índice de documentación](../README.md).
