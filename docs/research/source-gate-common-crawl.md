# Gate de fuente pasiva: Common Crawl Index

Fecha de verificación: 16-10-2026. Responsable: Jhojan. Revisor propuesto: César.
Estado del gate: **bloqueado para conector real; fixture sintético declarado**.
No se ejecutó ninguna consulta contra objetivos ni contra el índice para redactar este documento.

## Fuentes primarias consultadas

| Fuente                                                  | Fecha de consulta | Afirmación (claim)                                                                                                                                                                                                    | Limitación                                                                                                                                                                                   |
| ------------------------------------------------------- | ----------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [Índice CDX público](https://index.commoncrawl.org/)    | 16-10-2026        | Expone un índice URL/CDX por colección con API documentada (PyWB CDX) y lista de colecciones en JSON.                                                                                                                 | Cobertura histórica y parcial por colección; el operador pide no sobrecargar el servidor y derivar el volumen masivo a descargas o al índice columnar.                                       |
| [Términos de uso](https://commoncrawl.org/terms-of-use) | 16-10-2026        | Licencia limitada de acceso/uso; el contenido rastreado puede tener términos propios de sus dueños; se exige cumplir la ley aplicable y respetar derechos de terceros. Revisión vigente indicada: 7 de marzo de 2024. | Sin garantía de veracidad, exactitud, calidad o legalidad del contenido; el operador puede restringir o terminar el acceso; recomienda asesoría legal antes de usos sensibles o comerciales. |

## Evaluación contra el gate del [ADR 0010](../adr/0010-source-integration.md)

| Criterio del gate                | Resultado                                                                                                                    |
| -------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Proveedor y formato              | Common Crawl Index, respuesta CDX por colección. Formato documentado públicamente.                                           |
| Términos revisados               | Sí, a nivel de lectura (ver tabla). Falta la revisión legal del uso concreto con datos de un objetivo autorizado.            |
| Permiso para consultar objetivos | No existe. No hay dominio ni IP de prueba autorizado con par Nmap correspondiente.                                           |
| Dataset correspondiente a Nmap   | No existe. El fixture Nmap usa loopback y `.invalid`; ninguna fuente pública devolverá honestamente esa pareja.              |
| Cuota y límites observados       | No medidos. El operador publica pautas de uso responsable, pero sin un caso autorizado no hay cuota observada que registrar. |
| Frescura                         | No aplicable sin colección objetivo elegida.                                                                                 |
| Estrategia de fixture            | Fixture sintético rotulado como sintético. No simula una consulta real ni mide cobertura.                                    |
| Impacto en scope                 | Ninguno. La fuente no consultada no puede ampliar el scope.                                                                  |

## Veredicto

El conector real queda **bloqueado** hasta que exista un dominio o IP de prueba autorizado, su XML Nmap correspondiente, permiso documentado para consultar al proveedor, términos y retención aceptados, y cuota observada. Mientras tanto la demo usa el [fixture Nmap](../../fixtures/nmap/lab.xml) y datos sintéticos declarados. Esto cubre los requisitos existentes de fuente bloqueada y de no inventar resultados pasivos.

## Seguimiento

Registrar este veredicto en [decisiones abiertas](../process/open-decisions.md) y reabrir el gate solo con un objetivo autorizado y su evidencia de permiso. Ver también la [selección de fuentes](open-osint-selection.md), la [matriz de permisos y retención](osint-permissions-matrix.md) y el [alcance](../product/scope.md).
