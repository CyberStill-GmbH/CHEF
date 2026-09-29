# Factibilidad del corte web de octubre

Fecha 29-09-2026. **Datos confirmados:** presentación 31-10-2026; César, Diego y Jhojan disponen de 10 h semanales **cada uno**; prototipo web privado desplegado para el equipo es la meta deseada; Nmap XML autorizado sí está disponible; fuente OSINT pasiva/dataset correspondiente todavía no. React/TypeScript, **API Node.js con PostgreSQL/Prisma y GitHub OAuth, motor Go de correlación** son elecciones del propietario. Vercel/Railway son plataformas a las que se tiene acceso, no infraestructura configurada ni presupuesto aprobado.

## Capacidad y desfase

Periodo de trabajo 29-09 a 30-10, ~4.57 semanas. Tres personas × 10 h/semana ≈ **137.1 h brutas**. Reservar 25 % (34.3 h) para reviews, integración, ensayos e imprevistos deja **102.9 h netas** de planificación. Ceremonias y revisiones se cuentan dentro de la disponibilidad, no como trabajo invisible. Horas previas invertidas en núcleo/CLI no se vuelven a cargar; tampoco equivalen a web terminada.

Estimación inicial basada en historias del [Product Backlog](../plan/backlog.md), **no benchmark de implementación**: vertical web local con Nmap + fixture pasivo sintético + relación/UI/DB **110–160 h** incrementales; OAuth, aislamiento, despliegue privado y operación **70–120 h adicionales**. El objetivo original rondaba **180–280 h** antes de ampliar proveedores o activo; **no incluía contrato, build y conformidad Node–Go**, cuyo coste debe estimarse en el gate 05-10. Frente a 102.9 h ya había un déficit de al menos **77 h** para el mínimo privado, y ahora será mayor. No prometer que cabe por repartir tareas en sprints; Scrum inspecciona y adapta, no crea horas.

Opciones a decidir en gate 05-10: (A) mantener 31-10 y recortar a web local/sintética explicada, reconociendo que no cumple prototipo privado; (B) añadir capacidad real suficiente y revisar tamaño tras spikes; (C) mantener alcance privado y mover fecha. El propietario eligió B como **objetivo de producto** (web privada), no ha confirmado horas adicionales ni fecha nueva; por tanto la viabilidad del compromiso sigue abierta. No bajar silenciosamente a CLI ni simular una fuente OSINT viva.

## Incertidumbres a medir primero

1. **Fuente/dataset:** [Common Crawl Index](https://index.commoncrawl.org/) ofrece acceso público documentado, pero el fixture `.invalid` no puede enlazarse honestamente a datos reales. Jhojan verifica términos, cobertura, formato, cuota, fecha y un dataset de dominio autorizado. Sin pareja, usar fixture sintético explícito y no medir cobertura real.
2. **Contrato y correlación Go:** César/Jhojan etiquetan 5–10 casos positivos y negativos (servicio duplicado, hostname compartido, dato histórico, IP compartida) antes de ampliar el schema. Spike de transporte Node–Go, salida inválida, timeout, empaquetado Railway y conformidad frente al core TS; estimar horas observadas. Medir falsos enlaces y pérdidas de evidencia. Puntuación probabilística se posterga sin corpus calibrado.
3. **API/DB/OAuth:** César hace spike de migración y aislamiento en dos proyectos; después de GitHub OAuth, prueba callback, sesión y autorización de export. Acceso a Railway/Vercel no reduce estas pruebas.
4. **UI y rendimiento:** Diego valida grafo y lista sobre 100/1 000 nodos sintéticos con dispositivo/versiones medidos; mide tiempo para abrir relación/evidencia, no solo FPS. No prometer <1 s sin medición.
5. **Valor diferencial:** rúbrica y orden contrabalanceado CHEF/manual; tiempo, pasos, aciertos, falsos enlaces y muestra real. Umbral propuesto de ≥30 % reducción mediana sin más errores es hipótesis, no resultado.

Cada spike tiene timebox 2–4 h, artefacto y decisión. Si falla, se modifica el backlog antes de implementar dependencias. El [Scrum reestimado](../proposals/2026-09-scrum-to-oct31.md) usa gates; la [selección OSINT](open-osint-selection.md) distingue software abierto de datos/API.

## Costes y riesgos operativos

Hay potenciales gastos de hosting, PostgreSQL, almacenamiento, egress, OAuth, backups y proveedor OSINT que **no se han verificado**. El binario Go añade build, límites y observabilidad. No usar datos de clientes para descubrir presupuesto. Railway/Vercel se pueden estudiar con fixtures sintéticos y configuración sin secretos en Git. El despliegue multiusuario exige aislamiento/retención/backup/restore/observabilidad; sin ellos es una demo local, no un servicio confiable. [CWE/ASVS](../security/cwe-controls.md) y [ADR 0011](../adr/0011-node-go-boundary.md) recogen controles.

El spike Burp existente no está probado manualmente en Burp; su roadmap es posterior y no compite por las ~103 h salvo decisión explícita. La CLI sigue siendo fallback reproducible y prueba de regresión, no producto final elegido.
