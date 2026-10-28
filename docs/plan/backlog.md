# Backlog del producto CHEF

Antecedente actualizado el 18-10-2026. Este backlog describe la dirección web que el propietario retiró al escoger escritorio Tauri/Rust. Se conserva para rastrear estimaciones y decisiones previas; sus historias no son el plan de trabajo actual. El [backlog de migración a escritorio](backlog-desktop.md) y el [plan Scrum vigente](scrum-actual.md) contienen la prioridad activa.

## Reglas de lectura

- **Estado:** `hecho-core` significa que ya existe y tiene pruebas; `listo` exige fuente, permiso, contrato y Given/When/Then; `bloqueado` necesita una decisión externa; `propuesto` aún no se planifica como terminado; `parcial` identifica una entrega incompleta cuyo resto está descrito.
- **Tamaño:** rango exploratorio de esfuerzo incremental del equipo **incluyendo pruebas, revisión y documentación**, no horas registradas. Las historias de más de 8 h se parten al entrar al sprint. Una prueba técnica acotada puede producir conocimiento, no funcionalidad.
- **Prioridad:** P0 resuelve incertidumbre/riesgo; P1 hace usable la vertical; P2 completa la operación; P3 es evolución tras octubre. Una dependencia no satisfecha impide mover una historia a `listo`.
- **Corte de octubre:** prototipo privado desplegado para el equipo es el objetivo pedido, pero la suma de vertical y operación supera ~103 h netas. El gate del 05-10 debe registrar recorte, horas extra o cambio de fecha; [factibilidad](../research/feasibility.md) lo cuantifica.

## Inventario priorizado

| ID   | Épica / resultado verificable                                | Prioridad/estado | Responsable → revisor  |    Esfuerzo | Depende de                                     | Sprint candidato |
| ---- | ------------------------------------------------------------ | ---------------- | ---------------------- | ----------: | ---------------------------------------------- | ---------------- |
| WB01 | Validar ADR 0011 y prueba de frontera Node–Go/Prisma         | P0/propuesto     | César → Diego          |   reestimar | núcleo actual                                  | S1               |
| WB02 | Validar Common Crawl: acceso, permiso, términos y fixture    | P0/bloqueado     | Jhojan → César         |      6–10 h | fuente/autorización                            | S1               |
| WB03 | Acordar corte de demo y presupuesto Vercel/Railway           | P0/bloqueado     | César + equipo → Diego |       3–5 h | capacidad/hosting                              | S1               |
| WB04 | Contrato TS↔Go y observación/relación multifuente            | P1/propuesto     | César → Jhojan         |   reestimar | WB01,WB02                                      | S1–S2            |
| WB05 | Proyecto/membresía y repositorio Prisma/PostgreSQL           | P1/propuesto     | César → Diego          |     16–26 h | WB01,WB04                                      | S2               |
| WB06 | API de importación Nmap reutilizando núcleo y CLI intacta    | P1/propuesto     | César → Jhojan         |     12–20 h | WB04,WB05                                      | S2               |
| WB07 | Limpieza Nmap con recuento/motivo y controles negativos      | P1/propuesto     | Jhojan → César         |      8–14 h | contrato vigente                               | S2               |
| WB08 | Adaptador Common Crawl con procedencia/frescura/error        | P1/bloqueado     | Jhojan → César         |     12–22 h | WB02,WB04                                      | S2–S3            |
| WB09 | Motor Go: correlación/candidatos/contradicciones versionadas | P1/propuesto     | César → Jhojan         |   reestimar | WB04,WB06,WB08                                 | S3 o posterior   |
| WB10 | React: explorador de snapshot, lista/estado y evidencia      | P1/parcial       | Diego → César          |     14–22 h | contrato snapshot; API futura para importación | S3–S4            |
| WB11 | Mapa + lista accesible, filtros y explicación                | P1/propuesto     | Diego → Jhojan         |     16–28 h | WB09,WB10                                      | S3–S4            |
| WB12 | Snapshot/exportación de datos versionada y descarga segura   | P1/propuesto     | César + Diego → Jhojan |      8–16 h | WB04,WB09                                      | S4               |
| WB13 | GitHub OAuth, sesiones, roles e IDOR entre proyectos         | P2/propuesto     | César → Diego          |     16–28 h | WB05                                           | S4 o posterior   |
| WB14 | Despliegue privado, migración, secretos y recuperación       | P2/bloqueado     | César + Diego → Jhojan |     20–36 h | WB03,WB05,WB10,WB13                            | S4 o posterior   |
| WB15 | Estudio comparativo con pentesters, tiempo/pasos/errores     | P2/bloqueado     | Diego → César          |      8–16 h | WB08–WB12, piloto                              | S4 o posterior   |
| WB16 | Historial temporal, refresco medido y estado de proveedor    | P3/propuesto     | César + Jhojan → Diego | por refinar | evidencia de uso                               | después          |
| WB17 | Conectores adicionales, uno por proveedor y licencia         | P3/propuesto     | Jhojan → César         |  por fuente | WB02,WB08                                      | después          |
| WB18 | Proceso activo con alcance aprobado y control de salida      | P3/bloqueado     | César → Jhojan         | por refinar | nuevo ADR/autorización                         | después          |
| WB19 | BApp Montoya autónoma o cliente opcional                     | P3/propuesto     | César + Jhojan → Diego | por refinar | producto web validado                          | después          |

La interfaz de exploración de snapshot ya está implementada en `main`: fixture offline, validación contractual, lista de activos, panel de evidencia y estados accesibles. La API, grafo interactivo, revisión manual de accesibilidad y validación completa del recorrido siguen pendientes; por eso WB10 es parcial. Los rangos originales de WB04–WB15 ya excedían las ~103 h netas estimadas para octubre; **el contrato y motor Go agregan trabajo aún no estimado**. No adjudicar todas las historias al sprint ni reutilizar la cifra vieja de 152 h como total nuevo. WB02 y WB03 siguen bloqueadas por datos externos. C01–C04 del plan inicial ya se materializaron en el core y se mantienen como regresión.

## Criterios Dado/Cuando/Entonces por épica

**E1 · datos autorizados (WB02, WB04, WB07, WB08).** Dado un XML Nmap y una fuente pasiva permitida, cuando se importan, cada observación conserva origen, localizador, resumen criptográfico, fecha, alcance y versión; los duplicados/exclusiones se cuentan con motivo. Dado un archivo malicioso, una cuota agotada o una fuente sin acceso, cuando falla la importación, se informa el error y no aparecen observaciones inventadas ni resultados parciales. Si solo hay fixture sintético, la interfaz/demo lo declara.

**E2 · correlación explicable (WB09).** Dado el mismo servicio Nmap en dos registros y una señal pasiva fechada, cuando se comparan claves tipadas, la identidad fuerte se fusiona sin perder observaciones y la señal externa se enlaza como observada, inferida o candidata según la regla. Dado un nombre de host/IP compartido o una señal histórica incompatible, cuando se compara, no se atribuye propiedad ni vulnerabilidad y se conserva el control negativo.

**E2b · frontera Node–Go (WB01, WB04, WB09).** Dado un trabajo versionado y el mismo fixture, cuando Node invoca Go dos veces, el resultado, los IDs y las referencias son estables y la CLI TS no cambia. Dado un tiempo agotado, cierre inesperado o salida inválida, cuando falla Go, Node no publica relaciones parciales ni evidencia de otro proyecto. Antes de marcar WB09 como lista se necesitan esquema, ejemplo, validadores TS/Go, pruebas de conformidad y estimación de empaquetado.

**E3 · trabajo de analista (WB10–WB12).** Dado un proyecto con servicios y enlaces, cuando el usuario selecciona un nodo o una fila, ve la misma relación, regla, evidencia, fecha y motivo; puede filtrar sin cambiar el dato y exportar JSON versionado. Dada una lista vacía, evidencia rota o etiqueta HTML maliciosa, cuando se representa, la interfaz muestra un estado/error seguro, accesible por teclado, sin ejecutar contenido importado.

**E4 · multiusuario/operación (WB05, WB06, WB13, WB14).** Dados dos usuarios y dos proyectos, cuando uno cambia IDs de URL/cuerpo o intenta exportar el proyecto ajeno, el servidor deniega el acceso sin filtrar datos. Dado un XML inválido o un fallo de base de datos, cuando se importa, la transacción se revierte y la CLI actual sigue pasando las pruebas. El despliegue queda terminado solo con callback de OAuth, migración, secretos, aislamiento y recuperación ensayados en Vercel/Railway.

**E5 · valor medido (WB15).** Dada la misma tarea y datos autorizados, cuando los evaluadores alternan CHEF y revisión manual, se registran tiempos, pasos, aciertos, falsos enlaces y tamaño de muestra; también se publica un resultado adverso. No usar la cantidad de nodos como sustituto de productividad.

## Refinamiento, backlog del sprint y cambios

El responsable de producto ordena por valor y riesgo; el equipo estima y negocia capacidad. Solo se eligen historias listas que apoyen el objetivo del sprint y quepan con revisión y ceremonias. El [Daily Scrum](daily-scrum.md) organiza la conversación diaria; el [plan vigente](scrum-actual.md) asigna resultados a las tres personas, sin afirmar horas ya realizadas. En la reunión diaria se comparten `resultado / siguiente paso / bloqueo` y se adapta la secuencia cuando falla una dependencia. Una historia no se marca terminada por tener código: exige pruebas pertinentes, `npm run check`, revisor distinto, contrato/ADR actualizado y evidencia en el entorno declarado. Las historias web no desplazan la regresión del núcleo ni la autorización de red.

Después del primer conector, RDAP y RIPEstat se refinan como **issues separadas** con términos, fixture, límite y utilidad para Nmap propios; CT es importación posterior. Ninguna se considera incluida automáticamente en WB08.
