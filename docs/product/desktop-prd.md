# Documento de requisitos de producto: CHEF de escritorio

Actualizado 18-10-2026. Este documento expresa la dirección de producto aceptada por el propietario. La arquitectura requiere **revisión técnica** y la aplicación aún no está implementada.

## Problema

Durante una evaluación autorizada, el analista reúne resultados de reconocimiento y señales pasivas en distintas herramientas, pierde tiempo identificando duplicados y necesita rastrear por qué dos observaciones aparecen relacionadas. Un flujo local debe mantener la evidencia y permitir al analista decidir qué revisar, sin tratar coincidencias débiles como vulnerabilidades.

## Usuarios y resultado

El usuario principal es un pentester que trabaja con alcance autorizado y puede usar CHEF de forma local, incluso sin conexión a Internet. Debe poder abrir un proyecto, importar un resultado Nmap existente, revisar activos y evidencias, guardar el snapshot y exportarlo. Una tarea futura puede consultar una fuente pasiva o sincronizar un resultado seleccionado a DefectDojo con consentimiento explícito.

La hipótesis de valor es reducir los pasos y el tiempo para encontrar y explicar un servicio manteniendo falsos enlaces y evidencia perdida en cero o por debajo de una rúbrica acordada. Ningún beneficio se considera validado hasta medirlo con analistas y registrar muestra, tareas, tiempos, errores y limitaciones.

## Alcance del primer corte

- Aplicación de escritorio Tauri v2 para Windows/macOS/Linux, con objetivo de build por plataforma.
- Frontend React/Vite/TypeScript; Tailwind y shadcn/ui se incorporan tras definir tokens/componentes y dependencias.
- Núcleo de dominio y casos de uso Rust; Tokio para trabajos y cancelación.
- Persistencia local SurrealDB embebida con RocksDB para snapshot por `runId`; la primera adaptación CHEF guarda dominios, IP, servicios, endpoints, tecnologías, certificados, candidatos y evidencia tipada. Consultas de grafo, migraciones multiplataforma y backup siguen pendientes.
- xterm.js presenta logs estructurados. La primera tarea visible es un simulador offline con fixtures y etiqueta “Simulación”.
- Importación/exportación compatible con `Snapshot 1.0.0` y preservación de IDs, evidencia y controles negativos.

## Fuera del primer corte

El primer corte no lanza escaneos DNS/HTTP, no consulta Shodan/Censys/VirusTotal, no transmite datos a DefectDojo, no añade cuentas multiusuario, OAuth, API pública, telemetría ni servicio remoto. Cada integración futura tiene su propio permiso, contrato, criterios de uso, modelo de amenazas y prueba de error.

## Experiencia principal

1. Abrir o crear un proyecto local.
2. Importar XML Nmap autorizado o un snapshot CHEF validado.
3. Ver lista de activos y seleccionar uno para revisar las observaciones y la cadena de evidencia.
4. Iniciar la simulación offline y ver su estado/logs en xterm.js; cancelar en cualquier momento.
5. Guardar los datos localmente y exportar un snapshot; el usuario ve el destino de archivo y confirma exportación.
6. En una fase posterior, elegir manualmente señales pasivas autorizadas o un envío DefectDojo, revisar el contenido y confirmar.

## Principios de confianza

Rust es la autoridad para identidad, reglas, alcance, almacenamiento y salida remota. La UI solo presenta datos validados. Un evento no confiable nunca se interpreta como HTML ni shell. Los secretos viven en un almacén del sistema operativo, no en `config.toml`, logs o repositorio. La base guarda IDs CHEF deterministas y referencias verificables. Un fallo de parsing, reglas o almacenamiento no publica resultados parciales como completos.

## Métricas de producto

Medir instalación y primer uso, tiempo/pasos de tareas, proporción de referencias de evidencia recorribles, falsos enlaces en negativos, tiempo de respuesta UI y capacidad de recuperación local. Evaluar cada plataforma y motor de almacenamiento por separado. No usar número de nodos como medida de utilidad. Mantener como pendiente toda métrica que no tenga muestra observada y protocolo reproducible.
