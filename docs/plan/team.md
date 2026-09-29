# Equipo y forma de trabajo

César: Product Owner y responsable técnico; contratos, core, integración, scope, mentoring y decisiones. Diego: frontend React/TypeScript, grafo/evidencia, accesibilidad; revisa contratos con César. Jhojan: código de parser/fixtures/normalización, export y benchmarks; trabajo incremental con ejemplos/tests y revisión acompañada.

## Presupuesto por sprint

| Persona | S1              | S2              | S3     | S4                       | Responsable de revisión                    |
| ------- | --------------- | --------------- | ------ | ------------------------ | ------------------------------------------ |
| César   | C02 6h + Q01 1h | W03 7h          | I03 7h | Q03 3h + Q04 4h + B01 3h | Revisa core de Jhojan y contratos de Diego |
| Diego   | W01 6h + Q01 1h | W02 7h          | W04 7h | Q03 3h + Q04 4h + Q05 4h | Revisa export/core por contratos y UX      |
| Jhojan  | C01 5h + C03 2h | C04 4h + I02 3h | Q02 7h | Q03 3h + Q04 4h + B01 3h | Revisa fixtures/criterios y hace pairing   |

Horas incluyen la revisión asociada y sincronizaciones dentro del tamaño de la historia. Capacidad neta persona S1–S3 7.5h y S4 ≈11.8h. No sumar reviews por fuera sin recalcular presupuesto.

Para Jhojan: C01 recibe un XML de un host y expected output; pairing inicial30 min con César; primer PR solo parsing de IP/port; segundo PR estados/límites; tercero integración. C03: fixture de duplicado y control negativo; C04: export schema con prueba; Q02: medición reproducible; B01: verificación/fixture HTTP junto con César. Nunca ponerlo solo a resolver threading/seguridad de red.

Definition of Ready: problema y requisito, aceptación éxito/fallo, fixture, alcance, reviewer, tamaño ≤7h (o dividir). Definition of Done en [testing](../testing/strategy.md).

Tablero mínimo Todo / In progress / Review / Done; máximo una historia en implementación por persona y dos en Review para el equipo. Si Review está lleno, revisar antes de empezar otra historia. No hay logins de GitHub de Diego/Jhojan confirmados; no se invitó a nadie ni se asignaron cuentas ficticias.
