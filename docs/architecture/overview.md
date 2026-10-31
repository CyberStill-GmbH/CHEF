# Arquitectura de CHEF: estado actual y destino propuesto

Actualizado 18-10-2026. Este documento describe **lo que existe**. La nueva meta de escritorio se describe en [desktop-tauri.md](desktop-tauri.md); la antigua [web propuesta](proposed-web.md) se conserva como antecedente. La separación evita que un diagrama futuro se interprete como entrega. El modelo [C4 oficial](https://c4model.com/diagrams) usa distintos niveles de zoom: contexto (personas/sistemas), contenedores (unidades ejecutables y almacenes) y componentes (responsabilidades dentro de un contenedor).

## Nivel 1 · Contexto del sistema actual

```mermaid
flowchart LR
  Analyst["Analista autorizado"] -->|"elige XML y policy"| CHEF["CHEF · incremento offline"]
  Nmap["Resultado Nmap XML<br/>archivo no confiable"] -->|"importación local"| CHEF
  CHEF -->|"Snapshot 1.0.0 con evidencia"| Analyst
  Burp["Burp Suite<br/>sistema externo"] -.->|"spike Java separado, sin unión"| CHEF
```

CHEF actual es un proceso local de análisis, no un servicio SaaS. El analista controla el archivo y una allowlist literal de IP/puerto/protocolo; el XML jamás amplía ese scope. No se hacen conexiones al objetivo ni consultas OSINT. El spike Burp se muestra punteado porque no forma parte del flujo de importación del núcleo. Las flechas nombran interacciones y las cajas describen quién las controla.

## Nivel 2 · Contenedores actuales

```mermaid
flowchart TB
  Analyst["Analista"] -->|"comando local"| Tauri["Escritorio CHEF<br/>Tauri v2"]
  Tauri --> Web["React/Vite<br/>WebView"]
  Tauri --> Rust["Rust: core + aplicación<br/>workers Tokio"]
  Rust --> Store["SurrealDB embebida<br/>RocksDB"]
  Local["Archivos elegidos<br/>Nmap/JSONL/Censys/urlscan"] -->|"contenido acotado"| Tauri
  Web -->|"IPC tipado"| Tauri
  Tauri -->|"scan-log"| Web
  Analyst -->|"referencia CLI"| CLI["CLI Node/TypeScript<br/>offline"]
  CLI --> TS["Core TypeScript<br/>conformidad heredada"]
  Burp["Burp Suite"] -->|"selección HTTP"| Jar["JAR Montoya Java<br/>spike independiente"]
  Jar -->|"envelope HTTP 1.1.0"| Burp
```

El core TypeScript sigue siendo una referencia ejecutable **dentro del proceso Node**, no un servidor. El JAR Java vive en el proceso Burp y comparte fixtures/contratos, no imports TypeScript. El shell Tauri ya compone React, comandos Rust y almacenamiento local para importaciones offline; las respuestas archivadas de Censys y urlscan se persisten aparte de Snapshot y aparecen como pistas históricas. Las capacidades del catálogo todavía se portan por etapas. `apps/api` no tiene servidor; PostgreSQL/Prisma y OAuth no forman parte del destino local. El [diseño de escritorio](desktop-tauri.md) registra lo implementado, las fases pendientes y sus límites.

## Nivel 3 · Componentes del proceso Node

```mermaid
flowchart LR
  CLI["CLI: archivos, límites y composición"] --> UC["ImportEvidence<br/>caso de uso"]
  XML["Nmap XML no confiable"] --> Parser["NmapXmlParser<br/>adaptador"]
  Policy["Policy JSON"] --> Guard["ExactScopeGuard<br/>adaptador"]
  Parser -->|"registros normalizados"| UC
  Guard -->|"decisión de alcance"| UC
  UC --> Identity["Sha256Identity<br/>puerto de identidad"]
  UC --> Correlator["Correlación pura<br/>assets/relaciones"]
  Correlator --> Snapshot["Snapshot 1.0.0<br/>schema + referencias"]
  Snapshot --> CLI
```

La dependencia de código va desde CLI/adaptadores a casos de uso y dominio; el dominio no importa filesystem, red, React, Prisma ni Montoya. `ImportEvidence` recibe parser, guard, identidad y reloj por interfaces estrechas. El parser produce registros; no decide propiedad de activos ni persiste. El guard decide scope; la correlación crea IDs deterministas y conserva cada observación. La CLI publica JSON solo después de validarlo: un error deja stderr y ningún snapshot parcial. [SOLID](solid.md) y `npm run check:architecture` verifican dirección de imports, pero una revisión humana sigue siendo necesaria para responsabilidades semánticas.

## Fronteras de confianza e invariantes

1. **Archivo→parser:** bytes/XML son no confiables. CLI limita tamaño; parser rechaza DTD/entidades externas, profundidad y estructuras no soportadas. [CWE-611](https://cwe.mitre.org/data/definitions/611.html) y [CWE-400](https://cwe.mitre.org/data/definitions/400.html) orientan los controles; los fixtures hostiles son prueba, no garantía universal.
2. **Policy→scope:** solo IP/puerto/protocolo literales aprobados; exclusiones prevalecen. Ni un hostname en XML ni un proveedor OSINT pueden crear scope. El guard falla cerrado.
3. **Observación→inferencia:** evidencia tiene digest/locator/run; una relación guarda las observaciones que la sostienen. Un hash identifica contenido, no autentica origen. Hostname compartido no fusiona hosts; un servicio abierto no implica vulnerabilidad. `observed` e `inferred` son distintas aserciones.
4. **Snapshot→consumidor:** esquema y versión se validan antes de exportar; cualquier lector externo futuro también debe comprobar referencias únicas y existentes. UI no debe ejecutar HTML ni navegar URLs del dato importado ([CWE-79](https://cwe.mitre.org/data/definitions/79.html)).
5. **Escritorio e importaciones locales:** la ventana Tauri expone comandos IPC explícitos; la UI envía contenido de archivos seleccionados y la capa Rust vuelve a validar límites y contratos antes de almacenar. Las respuestas de proveedor son no confiables y se guardan separadas del Snapshot. No hay conectores remotos habilitados ni descubrimiento activo. La [matriz CWE](../security/cwe-controls.md), el [modelo de amenazas](../security/threat-model.md) y la [arquitectura de escritorio](desktop-tauri.md) especifican los controles pendientes.

## Cualidades y decisiones pendientes

**Correctitud:** mismo input + policy + reloj fijo producen IDs estables; control negativo impide una fusión débil. **Auditabilidad:** toda relación puede recorrerse hasta evidencia y linaje. **Disponibilidad:** límites de bytes/registros y cancelación cooperativa; las consultas activas siguen bloqueadas. **Migración:** el dominio TypeScript y sus fixtures siguen como referencia mientras Rust amplía la conformidad por componentes. El shell Tauri/React, IPC, grafo local y almacenamiento SurrealDB están implementados parcialmente; faltan capacidades del inventario, backups/recuperación multiplataforma y validación manual del instalador. No se habilitan llamadas externas por incluirlas en el diagrama.

Los [ADRs](../adr/README.md) registran decisiones y propuestas. El [ADR 0012](../adr/0012-escritorio-tauri-rust.md) describe el destino Tauri/Rust; su revisión del equipo sigue pendiente. El [modelo de amenazas](../security/threat-model.md) y [estándares](../governance/standards.md) completan los requisitos de calidad. El código, las pruebas y los contratos publicados prevalecen sobre un diagrama si discrepan.
