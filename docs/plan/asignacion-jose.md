# Asignación de trabajo · José

Fecha 30-10-2026. **Propuesta para aprobación del Product Owner; no es un rol concedido todavía.** José no figura en el [mapa de responsabilidades](team.md) ni tiene permisos en GitHub. Su alta y la creación de sus issues requieren decisión de César (PO) y registro en ese documento. Esta página **no reasigna** ownership de César ni de Diego: describe la parte que hoy no tiene dueño y que bloquea cualquier entrega.

## Por qué esta parte y no otra

El trabajo de producto que queda (API Node, PostgreSQL/Prisma, OAuth, motor Go, grafo) ya tiene dueño y está bloqueado por dependencias técnicas: sin [ADR 0011](../adr/0011-node-go-boundary.md) validado y sin contrato TS↔Go no hay nada que conectar, y sin API no hay nada que desplegar.

Lo que **nadie tiene asignado hoy** y bloquea la entrega completa es la cadena de entrega: el proyecto no existe como repositorio, el CI nunca se ejecutó, el build no está probado en una máquina limpia, no hay evidencia de ensayo y no hay paquete verificable. Eso es lo de José: **cadena de entrega y reproducibilidad**.

No requiere API, base de datos ni motor Go. No toca lógica de producto. Y desbloquea al resto: sin repositorio no puede haber PR, revisión cruzada ni checks verdes, que es justo lo que [AGENTS.md](../../AGENTS.md) y [commits-and-prs.md](../governance/commits-and-prs.md) exigen para dar algo por terminado.

**Ventana:** posterior al 31-10-2026. Nada de esta página se presenta como hecho en la demo. Si el PO aprueba el alta hoy, solo J1, J3 y J4 podrían alcanzar a influir en el ensayo; el resto es continuación.

## Estado de partida verificado (30-10-2026)

| Comprobación    | Resultado real                                                                  |
| --------------- | ------------------------------------------------------------------------------- |
| `npm run check` | EXIT=0 · 30 tests core + 34 tests web                                           |
| `npm run demo`  | EXIT=0 · Snapshot 1.0.0 válido                                                  |
| Repositorio Git | **No existe** `.git` → 0 commits, 0 PR, CI nunca ejecutado                      |
| `apps/api`      | Solo `README.md`; sin servidor                                                  |
| Motor Go        | 0 archivos `.go`                                                                |
| Prisma          | 0 `schema.prisma`                                                               |
| Toolchain local | Node v26 en la máquina vs. `engines: >=24 <25` → `npm ci` advierte `EBADENGINE` |
| Frontend        | Smoke: lista + panel de evidencia; sin grafo ni filtros                         |

Nada de lo anterior se corrige desde el código de producto. Se corrige desde la cadena de entrega.

## Alcance

**Dentro:** repositorio y flujo de PR, primera ejecución real de CI, toolchain reproducible, runbook de demo, evidencia de pruebas manuales, paquete de entrega firmado por hash, respaldo/restauración del repositorio.

**Fuera:** API, Prisma, OAuth, motor Go, correlación, contrato, grafo/mapa, diseño visual, fuente pasiva, cualquier despliegue de API o base de datos.

## Tareas

### J1 · Inicializar el repositorio y el flujo de PR

Prioridad P0 · 4–6 h · sin dependencias

Que el proyecto exista como historia versionada y que la revisión cruzada sea posible.

- **Given** el árbol actual sin `.git`, **When** se inicializa aplicando el `.gitignore` vigente, **Then** `git ls-files` no incluye `node_modules/`, `dist/`, `apps/web/dist/`, `artifacts/`, `.tools/`, `*.log` ni `.env*`, y el primer commit contiene todo el código y la documentación versionable.
- **Given** el commit inicial, **When** se revisa el diff completo, **Then** no aparecen credenciales, tokens, IPs internas ni el snapshot generado por la CLI.

**Entregable:** repositorio con rama `main` y primer commit según [commits-and-prs.md](../governance/commits-and-prs.md); `docs/governance/repo-bootstrap.md` con comandos ejecutados, SHA inicial, fecha y conteo de archivos versionados.

**Gate:** un tercero clona el repositorio, ejecuta `npm ci` y `npm run check`, y obtiene 0. Sin ese clon, J1 no está Done.

### J2 · Primera ejecución real de CI

Prioridad P0 · 3–5 h · depende de J1

Que "verde" signifique un run, no una intención.

- **Given** los workflows [Quality](../../.github/workflows/quality.yml) y [CodeQL](../../.github/workflows/codeql.yml) versionados pero nunca ejecutados, **When** se hace push a `main`, **Then** ambos aparecen en Actions con URL de run y SHA.
- **Given** un job en rojo por configuración (permisos, caché, versión de Node, matriz de lenguaje), **When** se corrige, **Then** el arreglo va en un PR con causa y evidencia, nunca en commit directo a `main`.
- **Given** que [ci-security.md](../governance/ci-security.md) exige la primera ejecución remota para validar permisos y SARIF, **When** el job termina, **Then** se documenta si Code Scanning quedó activo o si GitHub lo rechazó por plan o configuración.

**Entregable:** `docs/governance/ci-primera-ejecucion.md` con la tabla job → URL → conclusión → SHA, y la lista de checks que deben exigirse en la protección de `main` una vez conocidos sus nombres reales.

**Gate:** los nombres de check anotados son los que GitHub reporta, no los supuestos. Prohibido declarar "CI verde" sin URL.

### J3 · Toolchain reproducible

Prioridad P1 · 3–4 h · depende de J1

Que el build no dependa de la máquina de quien lo escribió.

- **Given** una máquina con Node distinto del declarado (`engines: >=24 <25` frente a v26 instalado), **When** se documenta la versión soportada, **Then** existe un archivo de versión coherente con `engines` y con el `node-version: '24'` del workflow.
- **Given** un clon limpio, **When** se ejecuta `npm ci`, **Then** no aparece `EBADENGINE`; y `npm run check`, `npm run demo` y `npm run web:build` terminan en 0.
- **Given** que hoy `npm ci` solo advierte, **When** se documenta, **Then** queda explícito que la advertencia es un riesgo y no se "arregla" relajando `engines`.

**Entregable:** `docs/governance/toolchain.md` y archivo de versión de Node.

**Gate:** log archivado de un `npm ci` limpio sin `EBADENGINE`.

### J4 · Runbook de demo ejecutable y evidencia del 31-10

Prioridad P0 · 4–6 h · depende de J1

Que la demo se pueda repetir sin el autor delante.

- **Given** que la [ruta de regresión](../demo/runbook.md) termina en la CLI offline y no menciona la web, **When** se amplía, **Then** incluye el paso de build y vista previa local del frontend y el recorrido de cinco minutos con qué se muestra en cada tramo.
- **Given** el guion, **When** lo ejecuta alguien que no escribió el código, **Then** completa el recorrido en menos de diez minutos sin ayuda y anota dónde se trabó.
- **Given** una afirmación del guion sin respaldo, **When** se revisa, **Then** se elimina o se marca como no implementada; el smoke no se presenta como producto ni como FR-04 cumplido.

**Entregable:** `docs/demo/runbook.md` ampliado y `docs/demo/evidencia-31-10.md` con entorno, SHA, comandos, salidas, capturas y límites declarados.

**Gate:** dos ensayos cronometrados con errores registrados.

### J5 · Paquete de entrega verificable

Prioridad P1 · 3–5 h · depende de J1 y J3

Que "esto es lo que entregamos" sea comprobable por un tercero.

- **Given** un commit de entrega, **When** se genera el paquete, **Then** contiene fuentes, `package-lock.json`, fixtures y documentación, más un archivo de sumas SHA-256 y una nota con SHA del commit, fecha y comandos de verificación.
- **Given** solo el paquete, **When** un verificador independiente recalcula los hashes y ejecuta `npm ci` y `npm run check`, **Then** obtiene el mismo resultado sin acceso a la máquina del autor.
- **Given** que el paquete se regenera en cada entrega, **When** se revisa el `.gitignore`, **Then** el artefacto no queda versionado.

**Entregable:** script de empaquetado documentado y nota de entrega.

**Gate:** restauración y verificación hechas por otra persona.

### J6 · Respaldo y restauración del repositorio

Prioridad P1 · 2–4 h · depende de J1

Que perder una máquina no signifique perder el proyecto.

- **Given** el repositorio con historial, **When** se crea un bundle completo y un remoto espejo, **Then** ambos se restauran en un directorio vacío.
- **Given** solo el bundle, **When** se clona, **Then** `git rev-parse HEAD` coincide con el original y `npm ci` y `npm run check` pasan.
- **Given** que **no existe base de datos**, **When** se documenta el respaldo, **Then** se dice explícitamente que cubre repositorio y artefactos, no PostgreSQL; no se acredita WB14.

**Entregable:** `docs/governance/backup-restore.md` con procedimiento y simulacro fechado.

**Gate:** simulacro de restauración ejecutado, no descrito.

### J7 · Registro de pruebas manuales pendientes

Prioridad P2 · 2–3 h · depende de J4

Que lo que ninguna prueba automatizada cubre quede visible en vez de supuesto.

- **Given** comportamiento sin cobertura automática (recorrido completo por teclado, fixture hostil, caso denegado entre proyectos, panel de evidencia sin `aria-live`), **When** se registra, **Then** cada fila tiene caso, entorno, resultado, fecha y responsable.
- **Given** una prueba no ejecutada, **When** se lista, **Then** queda como pendiente y no como aprobada.
- **Given** que el estudio con usuarios es de Diego (WB15), **When** se prepara este registro, **Then** se declara como insumo y no como sustituto.

**Entregable:** `docs/demo/pruebas-manuales.md`.

**Gate:** ninguna fila sin fecha ni entorno.

## Entregables

| Archivo                                   | Tarea | Qué demuestra                               |
| ----------------------------------------- | ----- | ------------------------------------------- |
| `docs/governance/repo-bootstrap.md`       | J1    | Repositorio creado con artefactos excluidos |
| `docs/governance/ci-primera-ejecucion.md` | J2    | CI ejecutado con URLs reales                |
| `docs/governance/toolchain.md`            | J3    | Build reproducible en versión declarada     |
| `docs/demo/runbook.md` (ampliado)         | J4    | Recorrido repetible por terceros            |
| `docs/demo/evidencia-31-10.md`            | J4    | Estado real del ensayo                      |
| Nota y script de empaquetado              | J5    | Entrega verificable por hash                |
| `docs/governance/backup-restore.md`       | J6    | Restauración probada                        |
| `docs/demo/pruebas-manuales.md`           | J7    | Pendientes declarados                       |

## Dependencias y bloqueos

- **GitHub:** organización, repositorio y permisos los concede César (PO). Sin eso, J2, J5 y J6 quedan en local.
- **Privacidad del repo:** si es privado, las URLs de Actions no son públicas; la evidencia se captura igual con captura de pantalla y SHA.
- **Nada de esta parte requiere** dominio autorizado, fuente viva, API ni base de datos. Por eso es ejecutable mientras el resto sigue bloqueado.

## Puertas de aceptación

1. `git ls-files` limpio de artefactos y secretos.
2. Dos URLs de run de CI con conclusión, no una promesa.
3. `npm ci` sin `EBADENGINE` en la versión declarada.
4. Runbook reproducido por un tercero y cronometrado.
5. Paquete verificado por alguien distinto del autor.
6. Restauración desde bundle ejecutada.
7. Cada afirmación de la demo con evidencia, o marcada como no implementada.

## Lo que NO es de José

| Área                                                                               | Dueño actual             | Por qué no se mueve                                                  |
| ---------------------------------------------------------------------------------- | ------------------------ | -------------------------------------------------------------------- |
| API Node, PostgreSQL/Prisma, OAuth y sesiones, motor Go, contrato TS↔Go            | César                    | Depende de WB01/WB04/WB05; sin esas piezas no hay nada que desplegar |
| Grafo/mapa, diseño visual, filtros, accesibilidad de producto, WB15                | Diego                    | Es su responsabilidad en [team.md](team.md) y requiere DTO aprobado  |
| Common Crawl real, matriz de permisos OSINT, benchmark dual-source, cierre de WB07 | Jhojan                   | Es ingestión, fixtures y calidad del dato                            |
| Despliegue privado de API/DB (WB14)                                                | César + Diego, bloqueado | Requiere WB03, WB05 y WB13; José no lo ejecuta ni se acredita        |

**Explícitamente prohibido en esta parte:**

- Cambiar contrato, esquema o reglas de correlación.
- Tocar `packages/domain`, `packages/application` o `packages/adapters`.
- Declarar "multiusuario", "desplegado" u "OSINT viva".
- Relajar `engines`, umbrales de CI o `--audit-level` para poner un job en verde.

## Reparto: José y nosotros

| José                                  | Nosotros (Jhojan)                                                |
| ------------------------------------- | ---------------------------------------------------------------- |
| Repositorio, ramas y flujo de PR      | Common Crawl contra dominio autorizado (WB02)                    |
| Primera ejecución de Quality y CodeQL | Adaptador CC con procedencia y frescura (WB08)                   |
| Toolchain y build reproducible        | `docs/research/osint-permissions-matrix.md`                      |
| Runbook y evidencia de ensayo         | Benchmark dual-source con fixtures                               |
| Paquete y respaldo                    | Cierre de WB07 con review de César                               |
| Registro de pruebas manuales          | Ampliación de DTO multi-source en `apps/web` con review de Diego |

El reparto es deliberado: José concentra lo que se puede cerrar **sin** desbloquear API, base de datos o fuente viva; nosotros concentramos lo que depende de datos autorizados y de contrato.

## Revisión

- José → reviewer: César para CI y seguridad; Jhojan para runbook y evidencia; Diego para accesibilidad del registro manual.
- Nosotros → reviewer: César para core y contrato; Diego para presentación de exclusiones.

Nadie revisa su propio trabajo, y ninguna de estas tareas se declara Done por existir el archivo.

## Ventana y esfuerzo

J1–J7 suman **21–33 h**, es decir dos a tres semanas a diez horas semanales. La parte es de tamaño acotado a propósito: es la que se puede terminar sin depender de decisiones que hoy no existen.

## Riesgos

1. **Alta sin registro.** Si José empieza sin estar en [team.md](team.md), su trabajo queda sin owner reconocido y sin reviewer asignado. Mitigación: César actualiza el mapa antes del primer PR.
2. **CI que no corre.** Si el plan del repositorio no habilita Code Scanning, hay que documentar el rechazo real y no simular verde cambiando umbrales.
3. **Restauración de papel.** Un procedimiento escrito no es un simulacro. El gate exige ejecutarlo.
4. **Presión de fecha.** El runbook puede presentar el estado real; no se compra tiempo con afirmaciones que el propio [scope.md](../product/scope.md) prohíbe.
