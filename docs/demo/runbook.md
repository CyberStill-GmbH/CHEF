# Runbook de demostración CHEF

Actualizado 29-09-2026. **Objetivo solicitado, no ruta ya construida:** prototipo web privado para el equipo en octubre. Recorrido: iniciar con GitHub → crear proyecto → importar Nmap autorizado → consultar fuente pasiva autorizada → revisar servicio y relación con dos evidencias/fechas → exportar. No lanzar escaneos desde CHEF. Antes de cada ensayo anotar commit, entorno, fuente, permiso, versión de reglas y qué pasos están implementados. Un fixture sintético se etiqueta como sintético; no se presenta como respuesta OSINT viva.

## Preparación y gates

1. Confirmar que `npm ci`, `npm run check`, build Java si aplica y CI están verdes. Preparar proyecto de prueba vacío y cuentas miembro/no miembro. Probar migración, backup/restore y callback OAuth **si** el despliegue privado existe.
2. Usar XML Nmap/alcance autorizados y la fuente pasiva aprobada con su política de retención; no utilizar datos de clientes en capturas o repo. El fixture [Nmap](../../fixtures/nmap/lab.xml) usa loopback/`.invalid`; sirve para fallback, no para una consulta OSINT real.
3. Preparar respuestas correctas: servicio duplicado con dos observaciones, hostname compartido que **no** fusiona hosts, fuente pasiva con fecha y estado candidato, archivo hostile que falla sin salida parcial. Un evaluador debe poder seguir Observation→Evidence→digest/locator y regla.
4. Ensayar en la URL privada desde navegador distinto y también el caso denegado de otro proyecto. Si seguridad o despliegue no están probados, **no** etiquetar el resultado como multiusuario/desplegado.

## Guion objetivo de cinco minutos

| Tiempo      | Acción que debe verse                  | Evidencia/claim permitido                                                        |
| ----------- | -------------------------------------- | -------------------------------------------------------------------------------- |
| 00:00–00:35 | Problema y proyecto autorizado         | Fuentes/fechas, objetivo de ahorrar revisión manual                              |
| 00:35–01:20 | Login/proyecto/importación Nmap        | Scope y resultado real, conteos de duplicados/exclusiones                        |
| 01:20–02:10 | Fuente pasiva elegida                  | Proveedor, permiso, fecha y error/cuota; sintética si aplica                     |
| 02:10–03:20 | Mapa/lista y servicio enlazado         | Dos cadenas de evidencia y regla; candidato ≠ identidad                          |
| 03:20–04:05 | Control negativo y estado ambiguo Nmap | Hostname compartido no fusiona; `open                                            | filtered`no es`open` |
| 04:05–05:00 | Exportación y contraste manual         | JSON/imagen solo si implementados; tiempo/pasos medidos, límites y próximo corte |

César explica contrato/regla/scope; Diego guía interacción y accesibilidad; Jhojan explica fixture, limpieza, procedencia y fuente. Los tres ensayan y se corrigen mutuamente. Grabar duración/errores/recuperación de al menos dos ensayos antes del 30-10; tres si la disponibilidad permite. No inventar resultados del experimento con pentesters.

## Ruta de regresión disponible hoy

El flujo **actual** verificable es CLI offline, sin API/DB/UI/Internet:

```sh
npm ci
npm run check
npm run build
node dist/apps/cli/src/main.js fixtures/nmap/lab.xml fixtures/scope/lab.json
```

Probar XML hostil `fixtures/nmap/hostile-xxe.xml`: se espera error sin JSON parcial. [Snapshot de respaldo](../contracts/examples/snapshot.json) permite explicar evidencia si la UI futura falla. Usar esta ruta como demostración de núcleo estable/fallback, **no** afirmar que cumple por sí sola el objetivo web actualizado. Registrar exactamente hasta qué paso llegó la vertical, con URL/commit y pruebas manuales realizadas/pendientes.
