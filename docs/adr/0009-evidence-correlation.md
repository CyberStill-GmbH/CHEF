# ADR 0009: Correlación multifuente explicable

Estado: **propuesto**. Fecha: 29-09-2026. Responsables propuestos: César (reglas), Jhojan (fixtures/negativos), Diego (explicación UI). Extiende [0002](0002-graph.md) sin cambiar identidad Nmap aceptada hasta que un contrato nuevo sea aprobado.

## Contexto e impulsores

El core solo importa Nmap; la identidad IP+protocolo+puerto y el control negativo de hostname compartido funcionan. El producto quiere enlazar un servicio observado activamente con datos OSINT pasivos y responder “qué sé, cuándo y por qué”. Datos de CT, RDAP o crawl son parciales/históricos y pueden contradecir Nmap. Falsos enlaces harían el mapa engañoso, aunque fuera vistoso. Requisitos: reproducibilidad, procedencia por fuente, protección de scope y revisión humana de incertidumbre.

## Alternativas

**A, fusión por string/hostname/IP:** barata, pero produce falsos positivos con virtual hosts, CDN, DNS cambiante o IP compartida. **B, reglas deterministas tipadas con candidatos:** fusión automática solo para clave fuerte definida, relaciones históricas y débiles como candidatos con razón y evidencia. **C, modelo probabilístico/ML:** [Fellegi–Sunter](https://www.tandfonline.com/doi/full/10.1080/01621459.1969.10501049) puede puntuar enlaces si hay corpus etiquetado, independencia/comparadores calibrados y umbrales evaluados; hoy esos datos no existen. [PROV-O](https://www.w3.org/TR/prov-o/) inspira procedencia, sin reclamar compatibilidad formal.

## Decisión propuesta

Elegir **B**. Guardar observaciones originales e inmutables con fuente, locator, digest, run, `observedAt`, scope y versión de parser. Normalizar claves por tipo y bloquear candidatos por proyecto/scope. El motor devuelve `observed`, `inferred`, `candidate` o `conflicted` con `ruleId/version`, evidencias a favor/en contra y justificación legible. No fusionar hosts por hostname, IP compartida, banner o certificado; mantener vínculo temporal sin declarar propiedad. Duplicados exactos se agrupan en la vista con conteo/motivo y referencias conservadas. Los [estados Nmap](https://nmap.org/book/man-port-scanning-basics.html) nunca se degradan a “abierto” por limpieza. Una exposición inferida no es vulnerabilidad confirmada.

## Consecuencias y evolución

Ventaja: reglas revisables, falsos enlaces detectables y exportación auditable. Coste: modelo de observación/relación más rico, semántica temporal, interfaz de revisión y tests multifuente. Cambiar el snapshot publicado exige schema/ejemplo/prueba/compatibilidad y nueva versión; no reinterpretar `1.0.0` silenciosamente. Un score probabilístico puede añadirse **como sugerencia**, nunca auto-merge, solo tras labels y evaluación de precisión/recobrado. El modelo temporal evita llamarlo “en tiempo real” sin medir frescura y latencia.

## Verificación

Given Nmap con duplicado y fixture pasivo de dominio/IP, When se importan en cualquier orden, Then el mismo servicio tiene las dos procedencias y la relación muestra regla/fechas. Given hostname compartido, CT histórico o dato de IP compartida, When se compara, Then queda candidato/no fusionado. Given evidencia contradictoria, When llega un run posterior, Then se conserva historia y estado `conflicted` en vez de sobrescribir. Medir falsos enlaces y tiempo para explicar relación contra revisión manual. El fixture pasivo sintético no valida cobertura de una fuente real. El [estudio](../research/recon-correlation-2026-09.md) describe algoritmo y métricas.
