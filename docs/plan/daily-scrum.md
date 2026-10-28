# Daily Scrum y coordinación diaria

Actualizado 18-10-2026. El [plan Scrum vigente](scrum-actual.md) contiene el objetivo, las tareas de César, Diego y Jhojan, los criterios de cierre y la capacidad. Esta página define el ritmo diario; no registra tareas como terminadas.

## Reunión diaria

El equipo se reúne hasta 15 minutos cada día laborable, a la hora que acuerde. Cada persona comunica:

1. Qué resultado verificable produjo desde el encuentro anterior.
2. Qué siguiente corte puede terminar en unas dos horas y cómo se comprobará.
3. Qué bloqueo o decisión impide avanzar.
4. Qué revisión necesita o debe hacer.

La conversación se centra en el avance hacia el objetivo del sprint y adapta el plan del día. No es un informe individual al Product Owner. Si una dependencia de fuente, permiso o contrato bloquea trabajo, el equipo registra la causa y trabaja en una revisión o fixture seguro que sí aporte al objetivo.

## Tablero y límites de trabajo

Usar las columnas `Listo → En curso → Revisión → Bloqueado → Hecho`. Cada tarjeta enlaza ID del backlog, persona responsable, revisor distinto, criterio Given/When/Then, dependencia y evidencia que permitirá cerrarla. Mantener una historia activa por integrante y un máximo de dos PRs pendientes de revisión para todo el equipo; si se llena la cola, revisar antes de abrir trabajo nuevo.

El trabajo de cada persona se reparte según [responsabilidades del equipo](team.md): César prioriza y lleva la arquitectura/API; Diego mantiene la interfaz y experiencia; Jhojan verifica ingestión, fixtures y trazabilidad. Priorizar no da permiso para omitir revisión ni para marcar como terminado algo que solo está propuesto.

## Coordinación para el cierre de octubre

- **19-10, revisión y planificación:** mostrar el explorador en local, revisar el estado real de `npm run check`, registrar validación manual y acordar capacidad para el corte final.
- **20–23-10, estabilización:** Diego corrige problemas del recorrido; César confirma el límite entre cliente de fixture y API futura; Jhojan comprueba procedencia y casos negativos. Toda tarea debe tener un revisor asignado.
- **24–27-10, revisión cruzada:** repetir checks, revisar accesibilidad y contenido hostil, actualizar backlog y documentar bloqueos/deudas.
- **28–30-10, ensayo y cierre:** ejecutar demo offline reproducible, recoger evidencia de checks y validación manual, congelar el corte el 30-10 y dejar tareas futuras en el backlog.

Si cambia la capacidad, el equipo ajusta primero la cantidad de trabajo. No se reducen los controles de alcance, evidencia, referencias, accesibilidad ni revisión. El [Scrum Guide](https://scrumguides.org/scrum-guide.html) describe el Daily Scrum como una instancia para inspeccionar el progreso y adaptar el plan.
