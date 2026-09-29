# Modelo de amenazas

Activos: credenciales del operador, datos de clientes, exactitud del grafo, estación de trabajo, disponibilidad de Burp y alcance autorizado. Adversario: archivo XML o tráfico HTTP controlado por un target no confiable; usuario accidentalmente configura scope demasiado amplio. No se asume que localhost sea automáticamente confiable.

| Amenaza / frontera                         | Mitigación requerida                                                                                          | Estado / prueba                                 |
| ------------------------------------------ | ------------------------------------------------------------------------------------------------------------- | ----------------------------------------------- |
| XXE/SSRF mediante XML                      | Bloquear DTD externo/subsets y entities; parser no hace red                                                   | Core / fixture hostile                          |
| Memoria/CPU por archivo o grafo enorme     | Lectura acotada 2 MiB, registros ≤10 000, profundidad XML ≤32; worker para ingestión interactiva              | Bytes/registros/profundidad core; worker futuro |
| Scope spoofing al importar                 | Validate policy, IP literal canónica, exclusión gana; archivo nunca amplia scope                              | Core / OUT_OF_SCOPE                             |
| DNS rebinding / redirect                   | Comprobar IP resuelta y scope al conectar y en cada salto; fijar conexión a IP aprobada con Host/SNI correcto | Requisito de futuros adaptadores activos        |
| Inyección shell/path                       | Sin ejecución de shell ni paths desde XML; CLI solo lee ruta elegida                                          | Core / revisión de adapters                     |
| Labels HTML/scripts en web/Swing           | Texto plano, sin innerHTML ni enlaces activos; saneamiento antes de UI                                        | UI pendiente; spike JTextArea                   |
| Falsa vulnerabilidad / evidencia fabricada | Assertion explícita, regla/version/hash/locator; no afirmar autenticidad de archivo                           | Core / contratos/control negativo               |
| Exfiltración de auth/tráfico               | No guardar headers/body/query por defecto; sin telemetría/terceros                                            | Core; spike normalizado                         |
| Abuso de futura API local                  | Loopback+token, Origin/CORS, upload bounded; no arbitrary URL/fetch                                           | API pendiente                                   |
| Supply chain                               | Lock exacto, audit, CI con permisos mínimos, revisar licencias                                                | CI/revisión de dependencias                     |
| Deadlocks/unload Burp                      | Background bounded executor, EDT para UI, catch/log, unload+release                                           | Spike + prueba manual pendiente                 |
| Corrupción por cancelación                 | Publicar snapshot solo tras validar; estados terminales únicos, transacción persistente                       | Core atomic; worker/API pendiente               |

## Decisiones de riesgo

El core sin red elimina varios caminos SSRF del incremento actual. No habilitar HTTP/DNS activo hasta implementar controles de conexión, redirección y cancelación. Importar un host y observarlo en un archivo no garantiza que siga siendo accesible.

La cancelación del core síncrono detecta señales ya activadas y en checkpoints. SIGINT no puede interrumpir inmediatamente el parse síncrono mientras el event loop esté ocupado; hay que migrar a worker antes de ofrecer cancelación interactiva durante ingestión. Se registra esta limitación en UI/API y backlog.

El benchmark usa material sintético: no estima tasa de falsos positivos en redes reales. Revisar amenazas cada vez que se añade red, almacenamiento, nueva fuente, third party o rendering de contenido no confiable.
