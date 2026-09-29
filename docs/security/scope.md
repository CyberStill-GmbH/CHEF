# Scope, límites y cancelación

Policy actual: direcciones IP literales exactas, puertos TCP/UDP y exclusions; arrays ordenados/normalizados; límite de archivo 2 MiB y 10 000 hosts/registros de puertos como techos. Schema limita el tamaño de configuración; CLI lee scope ≤64 KiB. No soporta rangos CIDR, hostname ni autorización indirecta.

El scope importa evidencia, no concede permiso para sondear red. La aplicación valida antes de correlacionar cada puerto open; una sola violación aborta todo el snapshot. Hosts down y estados closed/filtered/open|filtered no producen servicios open. DTD simple nmaprun permitida sin resolución; declaraciones externas/subsets se rechazan.

Para futuros módulos activos: allowlist explícita de hostname+IP/CIDR aprobados, puertos y protocolo; resolver al inicio y antes de cada conexión; no seguir redirect fuera de policy; repetir chequeo cada salto (máximo 3 inicialmente). CIDR privado no se permite solo por ser privado; metadata cloud, loopback y red institucional requieren scope explícito. Restringir a GET/HEAD inicialmente, sin forms/login/fuzzing.

Presupuestos activos propuestos: concurrencia4, 2 requests/s globales, timeout5 s, 30 s por job, body ≤256 KiB, máximo100 endpoints y profundidad2. No están implementados ni validados; incluirlos en tests del adaptador antes de activarlo.

Kill switch futuro: AbortController en API/worker, cancelar sockets/queue, checks antes de dispatch y hops, job cancelled terminal en <1 s; no enviar nada nuevo después. Core actual soporta preabort y checkpoints, con la limitación síncrona descrita en el modelo de amenazas.
