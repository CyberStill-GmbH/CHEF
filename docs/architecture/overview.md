# Arquitectura

Arquitectura de puertos y adaptadores con un monolito modular pequeño. C4 contexto representa el producto objetivo; contenedores punteados/indicados como plan todavía no existen. El núcleo actual funciona únicamente mediante CLI offline.

```mermaid
flowchart LR
  operator[Analista autorizado] --> chef[CHEF]
  fixture[Archivos locales no confiables] --> chef
  chef --> export[Snapshot con evidencia]
  chef -. futuro .-> lab[Laboratorio con scope explicito]
  burp[Burp y seleccion del operador] -. spike .-> chef
```

```mermaid
flowchart TB
  subgraph local[Equipo del operador]
    cli[CLI implementada] --> app[ImportEvidence]
    parser[Adaptador Nmap implementado] --> app
    scope[ScopeGuard implementado] --> app
    app --> core[Dominio y correlacion pura]
    core --> json[Snapshot JSON]
    api[API loopback planificada] -.-> app
    web[React TypeScript planificado] -.-> api
    sqlite[SQLite planificado] -.-> app
    montoya[Java Montoya spike separado] --> envelope[Observation HTTP propuesta v1.1]
  end
  untrusted[XML no confiable] --> parser
  json --> filesystem[Exportacion bajo control del operador]
```

## Flujo actual y confianza

CLI abre archivo regular y lee como máximo límite+1 bytes. Schema valida scope; normaliza IPv6. Parser convierte UTF-8/XML a registros, bloquea DTD externo/entidades y rechaza campos fuera de contrato. Application comprueba límites/cancelación y cada registro contra scope antes de crear snapshot. Correlación produce IDs deterministas y fusiona evidencias sin eliminar observaciones. CLI valida schema de salida y exporta sin almacenar raw ni hacer red.

Fronteras: filesystem/XML → adaptador; adaptador → dominio; JSON → futuro frontend; HTTP no confiable → Montoya. El hecho de que una request esté en Burp no hace confiable su contenido. Un hash identifica bytes; no prueba autenticidad del emisor.

## Evolución

Workers para parsing pesado/cancelación; puerto de repositorio para SQLite transaccional; API loopback autenticada para sesiones; SSE para diffs versionados; React aplica snapshots/diffs. Añadir estas piezas solo con historias y gates. No introducir microservicios, Neo4j ni un scheduler complejo para una demo de tres estudiantes.

La frontera Montoya comparte formatos/golden fixtures, no imports del core. Un subset Java duplica una implementación pequeña, pero obliga a pruebas de conformidad para impedir drift semántico.
