# ADR 0010: Fuentes OSINT por adaptador y activo solo importado

Estado: **propuesto**. Fecha: 29-09-2026. Responsables propuestos: Jhojan (conector/fixture), César (scope/seguridad), Diego (estado de fuente en UI). Extiende [0003](0003-ingestion.md); no autoriza red ni escaneos.

## Contexto y opciones

El único input operativo garantizado es Nmap XML sintético del repo. El propietario quiere fuentes OSINT abiertas y un prototipo web, pero no hay credenciales/permiso/dataset de dominio real confirmados. “Todas las APIs” mezcla proveedores con licencias, cuotas, tiempos y semánticas diferentes; no permite una prueba de Done estable. Opción A: integrar muchas fuentes de golpe. Opción B: adaptador pequeño por fuente, ordenado por permiso, procedencia y unión con Nmap. Opción C: importar outputs existentes sin consultar proveedores. La [selección documentada](../research/open-osint-selection.md) fija Common Crawl Index como primer conector candidato, RDAP y RIPEstat después, CT como importación posterior; ninguna está conectada hoy.

## Decisión propuesta

Elegir **B con C como fallback de demo**. Cada adaptador declara proveedor, versión/formato, términos, permiso, tipo de señal, timestamp, locator, límites/cuota, errores y estrategia de fixture. El caso de uso controla scope/proyecto; la fuente no puede ampliarlos. Las respuestas quedan como observaciones, no como activos confirmados. Fallo de proveedor, truncación o cuota agotada se muestran como `unknown/error`, nunca “no se encontró nada”. La primera vertical usa una sola fuente realmente autorizada; mientras falte, fixture sintético marcado. El activo en esta fase se limita a **importar resultados ya autorizados**: CHEF no lanza Nmap ni crawls/HTTP/DNS contra objetivos.

## Consecuencias y gates de seguridad

Menos amplitud inicial, mayor trazabilidad y mantenibilidad. No se envían dominios de clientes a terceros por defecto; se registra la selección de fuente/consentimiento y política de retención. Un futuro worker activo requerirá otro ADR con aprobación de scope, destinos/redirects, cuotas, aislamiento de red, cancelación y auditoría; [CWE-918](https://cwe.mitre.org/data/definitions/918.html) orienta el riesgo de SSRF. Los términos de un wrapper open source no cubren los de cada API subyacente. Acceso a Vercel/Railway tampoco autoriza peticiones a objetivos.

## Verificación

Given fuente aprobada y fixture con resultado/fecha/locator, When se importa, Then la evidencia conserva atribución y se correlaciona solo dentro de proyecto/scope. Given respuesta truncada, inválida, vencida o cuota agotada, When falla, Then se registra estado/causa sin fabricar observaciones ni afectar importación Nmap. Given un XML que menciona dominio/IP fuera de scope, When se procesa, Then ningún conector ni job lo consulta. Pruebas de red solo con permiso y entorno definidos; hasta entonces validación con fixtures. La [matriz CWE](../security/cwe-controls.md) indica controles adicionales.
