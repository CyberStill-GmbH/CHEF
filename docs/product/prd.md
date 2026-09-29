# PRD · CHEF

Actualizado 29-09-2026. Producto del **Centro Cultural de Ciberseguridad**. Visión confirmada por el propietario; implementación real delimitada abajo. [Scope](scope.md), [requisitos](requirements.md), [arquitectura](../architecture/overview.md) y [backlog](../plan/backlog.md) son documentos enlazados, no sustitutos del código/prueba.

## Problema y usuario

Un pentester autorizado recibe Nmap y otras señales de reconocimiento en formatos y momentos distintos. Repite limpieza de XML, salta entre herramientas, encuentra duplicados y debe justificar si un dominio, IP, puerto y servicio se relacionan. Una representación vistosa sin procedencia puede acelerar **errores**. Usuario inicial: analista/equipo pequeño que investiga un proyecto autorizado y prepara evidencia revisable; el operador de Burp es usuario de una fase posterior.

## Propuesta de valor y pregunta falsable

CHEF convierte resultados activos **importados** y OSINT pasiva autorizada en una superficie observada con reglas de correlación explicables. Prioriza qué cambió, qué está corroborado, qué se contradice y qué es solo candidato; preserva archivo/fuente, locator, hash, fecha, scope y versión de regla. Un mapa interactivo, lista accesible y panel de evidencia permiten responder “qué sabemos, por qué y cuándo”. Se exportan datos versionados y, más tarde, una vista visual segura. No se llama vulnerabilidad confirmada a un servicio abierto.

La **hipótesis diferencial** es que el analista complete tareas concretas de reconocimiento con menos tiempo/pasos y sin más falsos enlaces que revisando Nmap+fuente pasiva a mano. Protocolo propuesto: 5–8 evaluadores, mismo dataset autorizado, orden contrabalanceado, respuestas correctas fijadas antes, tiempo/pasos/aciertos/falsos enlaces/evidencia perdida. Meta inicial: ≥30 % menos tiempo mediano sin aumentar errores; **no está medido**. [Amass](https://github.com/owasp-amass/amass), [SpiderFoot](https://github.com/smicallef/spiderfoot), [Uncover](https://github.com/projectdiscovery/uncover) y BApps existentes ya agregan/visualizan datos; CHEF debe probar valor en **limpieza reversible y explicación de enlaces**, no afirmar originalidad por un grafo.

## Recorrido y límites de la primera experiencia

Meta de pantalla: GitHub OAuth → proyecto → importación Nmap con policy → consulta de una fuente pasiva seleccionada con permiso → mapa/lista de activos y relaciones → evidencia/fechas/regla/estado → export. React/TypeScript en Vercel y API TypeScript/Node + PostgreSQL/Prisma en Railway son arquitectura propuesta. “Tiempo real” requiere refresco medido, latencia y estado de proveedor; al principio mostrar “actualizado por importación” y fecha visible. CHEF no lanza escaneos en esta fase: solo importa resultados autorizados.

**Estado hoy:** núcleo TypeScript/Node con CLI offline, Nmap XML subset, scope, identidad, correlación conservadora y `Snapshot 1.0.0`; 11 tests. Un spike Java/Montoya separado normaliza requests seleccionadas pero no está unido al core ni validado manualmente en Burp. No existe web, API, DB, OAuth, conector OSINT core ni correlación multifuente. Los fixtures actuales usan loopback/`.invalid`; una fuente OSINT pública no puede devolver una coincidencia real con ellos. La fuente/dataset autorizado es la mayor dependencia.

## Éxito y calidad

- Toda entidad/relación del dataset de prueba resuelve observación, evidencia y regla; cero fusiones en controles negativos. Para dataset real, medir precisión/recall solo con etiquetas revisadas, no prometer valores antes.
- El usuario distingue observado, inferido, candidato y conflictivo; una señal CT/crawl histórica conserva su fecha y no se presenta como host actual.
- Dos usuarios/proyectos no pueden cruzar consulta, importación ni exportación. Una excepción de proveedor/DB/XML no publica datos parciales.
- El recorrido de demo se reproduce en entorno declarado; registrar 2–3 ensayos, fallos, recuperación y tiempo/pasos frente a proceso manual. Un fixture sintético demuestra lógica, no cobertura OSINT real.
- Desarrollo con contratos versionados, `npm run check`, CodeQL/CI, revisión humana y gates [CWE/ASVS](../security/cwe-controls.md). CodeQL no certifica ausencia de fallos.

La presentación objetivo es 31-10-2026 con 10 h/semana por César, Diego y Jhojan. El prototipo privado deseado supera las ~103 h netas según [factibilidad](../research/feasibility.md); el Product Backlog se recorta/reestima en el gate 05-10. No prometer BApp Store hasta completar primero el producto web, integración posterior y [validación PortSwigger](../bapp/acceptance.md).
