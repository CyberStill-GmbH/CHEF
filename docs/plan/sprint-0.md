# Sprint 0: preparación verificable

**Registro histórico del bootstrap inicial.** La etapa web multiusuario del 29-09-2026 fue retirada el 18-10-2026 al elegir la migración a escritorio Tauri/Rust. El [backlog de migración](backlog-desktop.md) y el [Scrum vigente](scrum-actual.md) son la planificación activa. No reabrir como trabajo nuevo tareas del core que ya están implementadas.

Este bootstrap se realiza en `docs` y se integra en `main` tras checks. No se contabiliza como trabajo terminado por los estudiantes; deben revisar y apropiarse de contratos y decisiones.

1. Repo público CHEF, descripción/topics, main inicial y rama docs solicitada.
2. Investigación con fuentes oficiales, PRD, scope real y criterios falsables.
3. Core/adapters/CLI, schemas, fixtures, pipeline y controles negativos.
4. CI, reglas de colaboración, contribución/seguridad, licencia y templates.
5. Spike Montoya con build; cargar en Burp sigue un gate manual explícito.
6. PR de preparación, CI y merge; conservar docs para trazabilidad.

Revisión del equipo: horas de 10 por persona confirmadas; faltan fuente pasiva autorizada, capacidad para la web privada, logins de compañeros y presupuesto/operación del hosting. La prioridad ya no se identifica como W01 aislada, sino como la vertical web seleccionada en gate WB01–WB03.

Antes de S1: configurar acceso de compañeros con sus logins confirmados; establecer máquina demo; ejecutar `npm ci`, `npm run check`, `npm run demo`; guardar la salida real y explicar un control negativo. No ejecutar scans institucionales sin autorización específica.
