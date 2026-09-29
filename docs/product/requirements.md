# Requisitos y trazabilidad

Prioridades M=Must, S=Should, L=Later. Estado describe este incremento; `plan` significa que todavía no existe implementación. Los presupuestos de rendimiento son propuestas a validar.

| ID     | Requisito / criterio comprobable                                                             | Prioridad / estado      | Historia / validación               |
| ------ | -------------------------------------------------------------------------------------------- | ----------------------- | ----------------------------------- |
| FR-01  | Importar UTF-8 Nmap, solo hosts up y puertos open; rechazar input inválido sin grafo parcial | M / core                | C01 / pipeline y CLI                |
| FR-02  | Cada activo/arista enlaza a Observation → Evidence → SHA-256/locator/run                     | M / core                | C02 / referencias + benchmark       |
| FR-03  | Mismo address/protocol/port fusiona entidad; hosts distintos no se fusionan por hostname     | M / core                | C03 / controles positivos/negativos |
| FR-04  | Mostrar grafo, lista accesible y razones de selección                                        | M / plan                | W01,W02 / UI teclado y fixture      |
| FR-05  | Exportar schema 1.0.0; rechazar versiones desconocidas y vulnerabilidad falsa                | M / core                | C04 / contrato + CLI                |
| FR-06  | Etiquetar exposición inferida sin afirmar exploitabilidad                                    | M / core                | C02,W02 / fixture y texto UI        |
| FR-07  | Correlacionar un endpoint HTTP con evidencia de servicio sin asumir DNS actual               | S / plan                | I02 / shared fixture TS/Java        |
| FR-08  | Importar selección Burp en scope y conservar evidencia normalizada                           | L / spike               | B01 / compilación + manual Burp     |
| NFR-01 | Sin red por defecto, sin llamadas a shell, sin scope implícito                               | M / core                | C01,Q01 / arquitectura y revisión   |
| NFR-02 | Input ≤ 2 MiB, ≤ 10 000 registros de puertos, scope ≤ 64 KiB; error definido                 | M / core                | C01,Q01 / límites adversariales     |
| NFR-03 | Dominio sin infraestructura; application depende de puertos                                  | M / core                | C02 / chequeo AST de dependencias   |
| NFR-04 | p95 core < 500 ms para 1 000 observations, UI < 1 s                                          | S / por medir           | Q02 / benchmark ampliado            |
| NFR-05 | Cancelar trabajo activo/batches < 1 s y sin nueva red tras cancelación                       | M / plan; preabort core | Q01 / worker/cancel integración     |
| NFR-06 | Instalación y uso sin Internet después de provisionar dependencias                           | M / core                | Q03 / ensayo desconectado           |
| NFR-07 | Lint/typecheck/tests/contratos/build/format pasan antes de merge                             | M / CI                  | Q01 / workflow Quality              |
| NFR-08 | Logs sin payload, credenciales ni rutas locales; retención explícita                         | M / core parcial        | Q01 / stderr CLI + política         |
| NFR-09 | Demo 5 min, tres ensayos y respaldo offline                                                  | M / plan                | Q03 / runbook                       |
| NFR-10 | Montoya sin dependencia de servicio obligatorio, unload limpio y threads acotados            | L / spike parcial       | B01 / pruebas manuales              |

No hay servidor HTTP ni semántica de borrado persistente aún. No se declara conformidad ASVS/SSDF por completar esta tabla.
