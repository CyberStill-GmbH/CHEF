# Alcance, límites y cancelación

## Política vigente para la migración de escritorio

La primera versión Tauri solo reproducirá fixtures locales y una simulación rotulada; no ejecutará DNS, HTTP ni sockets contra objetivos. La importación de evidencia autorizada no concede permiso de escaneo. Las consultas a fuentes pasivas y el envío a DefectDojo permanecerán desactivados hasta que el usuario configure credenciales y confirme cada integración y su destino. No se transmitirán secretos ni evidencia en logs.

La arquitectura y los presupuestos de los párrafos siguientes son requisitos propuestos para una fase futura. No habilitan conectores ni actividad de red por sí solos. La activación requerirá scope escrito, validación de destinos, límites, cancelación y pruebas revisadas.

Policy de importación actual: direcciones IP literales o redes CIDR canónicas, puertos TCP/UDP y exclusiones prevalentes; arrays ordenados/normalizados; límite de archivo 2 MiB y 10 000 registros como techos. El scope de importación filtra evidencia local; no ejecuta ni autoriza tráfico. No soporta hostname ni autorización indirecta.

El scope importa evidencia, no concede permiso para sondear red. La aplicación valida antes de correlacionar cada puerto open; una sola violación aborta todo el snapshot. Hosts down y estados closed/filtered/open|filtered no producen servicios open. DTD simple nmaprun permitida sin resolución; declaraciones externas/subsets se rechazan.

Para futuros módulos activos: allowlist explícita de hostname+IP/CIDR aprobados, puertos y protocolo; resolver al inicio y antes de cada conexión; no seguir redirect fuera de policy; repetir chequeo cada salto (máximo 3 inicialmente). CIDR privado no se permite solo por ser privado; metadata cloud, loopback y red institucional requieren scope explícito. Restringir a GET/HEAD inicialmente, sin forms/login/fuzzing.

El plan y el caso de uso HTTP Rust limitan a 100 orígenes, 3 redirects, 5 s por solicitud, 30 s por trabajo y 256 KiB de respuesta; el gate compartible aplica como máximo 4 solicitudes concurrentes y una separación mínima de 500 ms entre despachos. El ejecutor exige un puerto de autorización que revalide el scope y entregue IPs permitidas para cada origen/salto; `ReqwestHttpTransport` conecta solo a esas IP, devuelve el peer real y no sigue redirects, proxies ni reintentos automáticos. La ejecución incluye una traza en memoria por salto, vinculada a eventos fuente, digest del scope, IP permitidas y status recibido. No hay adaptador de autorización ni composición Tauri: el transporte no se ha invocado, y el comportamiento sigue sin validar. Ninguna de estas piezas activa tráfico por sí sola. La profundidad 2 continúa propuesta para otros flujos.

Kill switch futuro: AbortController en API/worker, cancelar sockets/queue, checks antes de dispatch y hops, job cancelled terminal en <1 s; no enviar nada nuevo después. Core actual soporta preabort y checkpoints, con la limitación síncrona descrita en el modelo de amenazas.
