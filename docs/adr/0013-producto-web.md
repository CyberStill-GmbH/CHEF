# ADR 0013: CHEF como producto web

Estado: **dirección vigente; transición técnica pendiente**. Fecha: 31-10-2026.

## Contexto

La instrucción más reciente del propietario es entregar CHEF como producto web, sin aplicación de escritorio como destino. Esto sustituye la dirección de producto del [ADR 0012](0012-escritorio-tauri-rust.md). El repositorio conserva código Tauri/Rust y una interfaz React/Vite; la versión que corre directamente en el navegador todavía usa `FixtureApiClient`. No existe aún una API HTTP de CHEF conectada a esta interfaz.

## Decisión

- La interfaz de uso se entrega como aplicación web React, TypeScript y Vite, adaptable a escritorio y móvil.
- Rust sigue siendo el lenguaje objetivo del backend CHEF. La aplicación web consumirá una API HTTP implementada y compuesta en Rust.
- La consola visible en el navegador es una interfaz de comandos CHEF acotada; no es una shell del sistema. Los flujos de reconocimiento activo permanecen deshabilitados hasta aprobar autorización y controles de scope.
- La UI debe distinguir datos de fixtures, datos persistidos y actividad transmitida. Una animación de distribución del grafo no se presenta como tráfico de red en vivo.
- Esta decisión no selecciona hosting, autenticación, base de datos web ni protocolo de streaming. Esas decisiones requieren sus propios ADR y criterios de aislamiento de proyectos.
- La implementación Tauri existente se conserva como código histórico durante la transición; no constituye el destino de producto ni prueba que la web esté conectada al backend.

## Consecuencias y transición

La web puede ejecutarse y compilar de forma independiente con Vite. El demo actual lee contratos/fixtures locales y se identifica como vista previa offline. Para reemplazar esa vista se debe implementar, en cortes pequeños, la API Rust para cargar/importar evidencia, leer grafo y procedencia, exportar informes y emitir progreso de tareas; luego se añaden identidad, autorización por proyecto, almacenamiento compartido, operación y despliegue. Hasta entonces no se debe presentar la experiencia web como multiusuario ni como backend operativo.

Se mantienen los contratos Snapshot y Recon Graph, los IDs deterministas, el linaje de evidencia y los controles negativos. Cualquier cambio de contrato requiere esquema, ejemplo, prueba y nota de compatibilidad. Los resultados de fuentes archivadas siguen siendo históricos o candidatos según su tipo; no se convierten en observaciones actuales.

## Criterios de aceptación

1. Dado el sitio CHEF servido en un navegador, cuando se abre con el cliente de demostración, la UI identifica los datos como fixture/offline y no afirma conexión con el backend.
2. Dada una API Rust disponible, cuando se importe evidencia con un proyecto autorizado, el backend valida entradas, conserva IDs/evidencia y devuelve estado/progreso con contratos versionados.
3. Dado un usuario sin permiso para un proyecto, cuando intenta leer, importar o exportar un recurso de otro proyecto, la API no revela ni modifica ese recurso.
4. Dado un cambio de tamaño o dispositivo, cuando se presenta el explorador, navegación, terminal y grafo siguen siendo legibles y operables con teclado y táctil.

Los criterios 2 y 3 están **propuestos, no implementados ni validados**. La revisión humana del equipo sigue pendiente.
