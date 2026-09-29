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

## Extensión propuesta para producto web multiusuario

La [arquitectura web propuesta](../architecture/proposed-web.md) añade navegador, GitHub OAuth, API, PostgreSQL/Prisma, proveedor OSINT y exportaciones. **Estos controles no están implementados todavía.** La [matriz CWE](cwe-controls.md) registra debilidad, prueba y estado.

- **Identidad ≠ autorización.** OAuth prueba quién inicia sesión; cada lectura/importación/exportación necesita membresía y acción del proyecto. Probar dos usuarios/dos proyectos, IDs cambiados y artefactos temporales (CWE-862/639). Las políticas RLS pueden reforzar, no reemplazar el control de aplicación.
- **Datos de proyecto y secretos.** Guardar solo evidencia necesaria, definir retención/borrado/backup, cifrado y clasificación; tokens de OAuth/proveedor quedan en servidor y no en logs/cliente. No enviar dominios de clientes a una API externa por defecto. Hash de evidencia no anonimiza datos ni autentica emisor.
- **Conector externo.** Configurar proveedores/destinos permitidos, no aceptar URL arbitraria; comprobar redirects, respuesta, tamaño, cuota, términos y tiempo. Si en el futuro se añaden jobs activos, aislar egress y validar scope aprobado antes de cada conexión; riesgo CWE-918. La fase actual solo importa resultados autorizados.
- **UI/export visual.** Tratar hostname, banner, path, descripción de proveedor y SVG descargable como datos hostiles. Renderizar texto, no HTML; controlar enlaces y contenido embebido. Probar etiquetas maliciosas y descarga (CWE-79).
- **Integridad semántica.** Un proveedor que falla no significa ausencia de activos. Mostrar estado, última actualización y contradicciones; no borrar observaciones al deduplicar ni inferir vulnerabilidad por puerto. Versionar reglas y conservar procedencia.

El despliegue privado tiene gate: OAuth/session, control entre proyectos, migración/rollback, backup/restore, logs seguros, límites y tests end-to-end con usuario no miembro. No se declara listo por tener CI verde o una URL. La revisión de amenazas acompaña cada nuevo adaptador/rol/worker.
