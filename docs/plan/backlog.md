# Product Backlog de CHEF

Actualizado 29-09-2026. Este es el **orden de valor y dependencia del producto web**; no es una promesa de que todas las historias terminen el 31-10. El [Sprint Backlog](../proposals/2026-09-scrum-to-oct31.md) selecciona un subconjunto según capacidad real. Las [issues sugeridas](../proposals/2026-09-issues.md) desarrollan escenarios; se crean/asignan en GitHub solo tras refinement. César es Product Owner/responsable de backend y correlación; Diego responde por frontend/UX; Jhojan por ingestión, fixtures e investigación de fuentes. Diez horas semanales por persona, revisión cruzada y una historia en curso por persona.

## Reglas de lectura

- **Estado:** `hecho-core` significa que ya existe y tiene tests; `ready` exige fuente, permiso, contrato y GWT; `blocked` necesita decisión externa; `proposed` aún no se planifica como Done.
- **Tamaño:** rango exploratorio de esfuerzo incremental del equipo **incluyendo tests, review y docs**, no horas registradas. Historias >8 h se parten al entrar al sprint. Un spike acotado puede producir conocimiento, no funcionalidad.
- **Prioridad:** P0 resuelve incertidumbre/riesgo; P1 hace la vertical usable; P2 completa operación; P3 es evolución tras octubre. Una dependencia no satisfecha impide mover a `ready`.
- **Corte de octubre:** prototipo privado desplegado para el equipo es el objetivo pedido, pero la suma de vertical y operación supera ~103 h netas. El gate del 05-10 debe registrar recorte, horas extra o cambio de fecha; [factibilidad](../research/feasibility.md) lo cuantifica.

## Inventario priorizado

| ID   | Épica / resultado verificable                                | Prioridad/estado | Responsable → reviewer |    Esfuerzo | Depende de             | Sprint candidato |
| ---- | ------------------------------------------------------------ | ---------------- | ---------------------- | ----------: | ---------------------- | ---------------- |
| WB01 | Validar ADR 0011 y spike frontera Node–Go/Prisma             | P0/proposed      | César → Diego          |   reestimar | núcleo actual          | S1               |
| WB02 | Validar Common Crawl: acceso, permiso, términos y fixture    | P0/blocked       | Jhojan → César         |      6–10 h | fuente/autorización    | S1               |
| WB03 | Acordar corte de demo y presupuesto Vercel/Railway           | P0/blocked       | César + equipo → Diego |       3–5 h | capacidad/hosting      | S1               |
| WB04 | Contrato TS↔Go y observación/relación multifuente            | P1/proposed      | César → Jhojan         |   reestimar | WB01,WB02              | S1–S2            |
| WB05 | Proyecto/membresía y repositorio Prisma/PostgreSQL           | P1/proposed      | César → Diego          |     16–26 h | WB01,WB04              | S2               |
| WB06 | API de importación Nmap reutilizando core y CLI intacta      | P1/proposed      | César → Jhojan         |     12–20 h | WB04,WB05              | S2               |
| WB07 | Limpieza Nmap con recuento/motivo y controles negativos      | P1/proposed      | Jhojan → César         |      8–14 h | contrato vigente       | S2               |
| WB08 | Adaptador Common Crawl con procedencia/frescura/error        | P1/blocked       | Jhojan → César         |     12–22 h | WB02,WB04              | S2–S3            |
| WB09 | Motor Go: correlación/candidatos/contradicciones versionadas | P1/proposed      | César → Jhojan         |   reestimar | WB04,WB06,WB08         | S3 o posterior   |
| WB10 | React: proyecto/importación/estado y evidencia               | P1/proposed      | Diego → César          |     14–22 h | WB04,WB06              | S1–S3            |
| WB11 | Mapa + lista accesible, filtros y explicación                | P1/proposed      | Diego → Jhojan         |     16–28 h | WB09,WB10              | S3–S4            |
| WB12 | Snapshot/exportación de datos versionada y descarga segura   | P1/proposed      | César + Diego → Jhojan |      8–16 h | WB04,WB09              | S4               |
| WB13 | GitHub OAuth, sesiones, roles e IDOR entre proyectos         | P2/proposed      | César → Diego          |     16–28 h | WB05                   | S4 o posterior   |
| WB14 | Despliegue privado, migración, secretos, backup/restore      | P2/blocked       | César + Diego → Jhojan |     20–36 h | WB03,WB05,WB10,WB13    | S4 o posterior   |
| WB15 | Estudio comparativo con pentesters, tiempo/pasos/errores     | P2/blocked       | Diego → César          |      8–16 h | WB08–WB12, piloto      | S4 o posterior   |
| WB16 | Historial temporal, refresco medido y estado de proveedor    | P3/proposed      | César + Jhojan → Diego | por refinar | evidencia de uso       | después          |
| WB17 | Conectores adicionales, uno por proveedor y licencia         | P3/proposed      | Jhojan → César         |  por fuente | WB02,WB08              | después          |
| WB18 | Worker activo con scope aprobado y control de egress         | P3/blocked       | César → Jhojan         | por refinar | nuevo ADR/autorización | después          |
| WB19 | BApp Montoya autónoma o cliente opcional                     | P3/proposed      | César + Jhojan → Diego | por refinar | producto web validado  | después          |

Los rangos originales de WB04–WB15 ya excedían las ~103 h netas de octubre; **el contrato y motor Go agregan trabajo aún no estimado**. No adjudicar todos al sprint ni reutilizar la cifra vieja de 152 h como total nuevo. WB02 y WB03 son bloqueos reales, no tareas “casi hechas”. C01–C04 del plan inicial ya se materializaron en el core y se mantienen como **regresión**, no se reabren para gastar capacidad.

## Criterios Given/When/Then por épica

**E1 · datos autorizados (WB02, WB04, WB07, WB08).** Given XML Nmap y una fuente pasiva permitida, When se importan, Then cada observación conserva origen, locator, digest, fecha, scope y versión; los duplicados/exclusiones se cuentan con motivo. Given archivo hostile, cuota agotada o fuente sin acceso, When falla, Then se informa error y no aparecen observaciones inventadas ni parcial. Si solo hay fixture sintético, la interfaz/demo lo declara.

**E2 · correlación explicable (WB09).** Given mismo servicio Nmap en dos registros y señal pasiva fechada, When se comparan claves tipadas, Then identidad fuerte se fusiona sin perder observaciones y la señal externa se enlaza como observado/inferido/candidato según regla. Given hostname/IP compartidos o señal histórica incompatible, When se compara, Then no se atribuye propiedad ni vulnerabilidad, y el control negativo queda separado.

**E2b · frontera Node–Go (WB01, WB04, WB09).** Given job versionado y mismo fixture, When Node invoca Go dos veces, Then resultado, IDs y referencias son estables y la CLI TS no cambia; Given timeout/crash/salida inválida, When falla Go, Then Node no publica relaciones parciales ni evidencia de otro proyecto. Antes de mover WB09 a Ready se necesitan schema, ejemplo, validadores TS/Go, conformance y estimación de empaquetado.

**E3 · trabajo de analista (WB10–WB12).** Given proyecto con servicios y enlaces, When el usuario selecciona nodo o fila, Then ve la misma relación, regla, evidencia, fecha y motivo; puede filtrar sin cambiar el dato y exportar JSON versionado. Given lista vacía, evidencia rota o etiqueta HTML hostil, When se renderiza, Then muestra estado/error seguro, accesible por teclado, sin ejecutar contenido importado.

**E4 · multiusuario/operación (WB05, WB06, WB13, WB14).** Given dos usuarios y dos proyectos, When uno cambia IDs de URL/cuerpo o intenta exportar el otro, Then el servidor niega acceso sin fuga. Given XML inválido o fallo DB, When se importa, Then transacción revierte y la CLI actual sigue pasando tests. Despliegue Done solo con OAuth callback, migración, secretos, aislamiento y recuperación ensayados en Vercel/Railway.

**E5 · valor medido (WB15).** Given la misma tarea y datos autorizados, When evaluadores alternan CHEF y revisión manual, Then se registran tiempos, pasos, aciertos, falsos enlaces y muestra real; un resultado adverso se publica. No usar número de nodos como sustituto de productividad.

## Refinement, Sprint Backlog y cambios

El Product Owner ordena por valor/riesgo; el equipo estima y negocia capacidad. Cada lunes se eligen solo historias `ready` que apoyen el objetivo de sprint y quepan con review/ceremonias. La [pauta diaria por persona](daily-scrum.md) muestra el trabajo esperado, no horas ya realizadas. En el Daily Scrum se actualizan `Done hoy / siguiente paso / bloqueo` y se renegocia la secuencia cuando falla un gate. Una historia no se mueve a Done por tener código: exige prueba positiva/negativa, `npm run check`, revisor distinto, contrato/ADR al día y evidencia en entorno declarado. Las historias web no desplazan la regresión del núcleo ni la autorización de red.

Después del primer conector, RDAP y RIPEstat se refinan como **issues separadas** con términos, fixture, límite y utilidad para Nmap propios; CT es importación posterior. Ninguna se considera incluida automáticamente en WB08.
