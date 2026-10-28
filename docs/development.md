# Desarrollo local

Esta guía prepara el checkout CHEF para inspección y compilación local. El flujo predeterminado usa datos sintéticos y permanece offline. No se necesita credencial, servicio externo ni objetivo de red.

## Responsables de subsistemas

- **César — backend e integración:** Rust/Tokio, validación de importación, comandos Tauri y coordinación de trabajos.
- **Diego — interfaz e IPC:** React/Vite/TypeScript, accesibilidad, contratos IPC y presentación de actividad.
- **Jhojan — persistencia y conformidad:** SurrealDB/RocksDB, fixtures, procedencia, recuperación y control de cambios de datos.

La asignación vigente y los criterios Dado/Cuando/Entonces están en [Scrum actual](plan/scrum-actual.md). La revisión de una historia corresponde a otra persona del equipo.

## Requisitos

- Node.js 24.x (el `package.json` declara `>=24 <25`) y npm.
- Rust stable con soporte para edition 2024 y Cargo.
- Para compilar escritorio en Windows: Visual Studio Build Tools con MSVC, CMake, NASM y `libclang`; usa las versiones y rutas del entorno local registrado en el [ADR 0012](adr/0012-escritorio-tauri-rust.md).
- El runtime WebView nativo de la plataforma para ejecutar la aplicación empaquetada.

No instales dependencias globales ni cambies la política de PowerShell para ejecutar los scripts. Si `npm.ps1` está bloqueado, invoca `npm.cmd`.

## Preparación y verificación estática

Desde la raíz del repositorio:

```powershell
npm ci
npm run check:docs
cargo fmt --manifest-path packages/rust-core/Cargo.toml -- --check
cargo check --manifest-path apps/desktop/src-tauri/Cargo.toml --locked --offline
```

`npm run check:docs` valida referencias, Mermaid, frontmatter, contratos y ejemplos. `cargo fmt` revisa formato; `cargo check` compila sin ejecutar la suite funcional. La suite completa se ejecutará cuando el propietario retire el aplazamiento; entonces se usará `npm run check`.

Para iniciar el escritorio en Windows con el toolchain nativo disponible:

```powershell
npm run desktop:dev
```

Para generar los instaladores de la plataforma actual:

```powershell
npm run desktop:build
```

La compilación no acredita el arranque ni la revisión del flujo WebView. Registra aparte los resultados manuales pendientes en `docs/process/verification.md`.

## CLI y frontend de referencia

El core TypeScript/CLI sigue siendo referencia de compatibilidad durante el portado:

```powershell
npm run build
npm run demo
```

El frontend web puede revisarse por separado:

```powershell
npm run web:dev
```

Los comandos de test y validación integral pertenecen a la fase posterior indicada por el propietario. No los marques como ejecutados si solo compilaste el proyecto.

## Datos y seguridad

Usa los fixtures versionados en `fixtures/`. Mantén los ficheros de alcance explícitos: la política de importación filtra datos locales y **no** autoriza consultas ni conexiones. No incluyas evidencia real, credenciales, `.env`, capturas de tráfico o logs sensibles en Git. La UI no tiene shell ni acceso directo al filesystem, la base o la red.

Antes de cambiar un contrato, revisa esquema, ejemplo, política de compatibilidad y consumidores. Conserva IDs deterministas, procedencia, estados negativos y relaciones de evidencia. Registra por separado lo implementado, lo compilado, lo probado automáticamente y lo validado manualmente.
