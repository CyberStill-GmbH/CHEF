# SOLID comprobable

| Principio | Aplicación actual                                                                     | Regla de revisión                                                                      |
| --------- | ------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- |
| SRP       | Parser convierte XML; guard decide alcance; correlator fusiona; CLI compone y hace IO | No introducir HTTP/filesystem en parser o dominio                                      |
| OCP       | ImportEvidence recibe ImportParser y ScopeGuard                                       | Nueva fuente mediante adaptador; revisar el modelo si introduce hechos nuevos          |
| LSP       | Los puertos definen retorno, errores y ausencia de efectos de red                     | Todo parser entrega registros normalizados o ChefError, nunca grafo parcial            |
| ISP       | IdentityPort y ClockPort pequeños; ImportParser separado de persistencia/red          | Evitar interface universal de plugin con métodos vacíos                                |
| DIP       | Application importa puertos/modelo de domain; CLI inyecta implementaciones            | `check:architecture` inspecciona imports con AST; dominio no importa paquetes externos |

No se usa herencia de clases para compartir lógica sin necesidad. Funciones puras para correlación; dependencias inyectadas donde son observables. Interfaces no son prueba automática de SOLID: revisar responsabilidades, invariantes y pruebas.

## Reglas del modelo

Asset/address tiene key IP canónica. Asset/service usa address|protocol|port. Evidence identifica archivo/versión/run; Observation preserva locator único. Dos observaciones del mismo servicio conservan dos referencias. Relationship solo fusiona endpoints/kind exactos. Reducir confianza o invalidar evidencia necesita nueva operación/versionado; el MVP no elimina hechos de un snapshot.

No deducir que hostname idéntico significa host idéntico; ni que dos hosts con misma IP son la misma aplicación. Futuro endpoint se identifica por origin, método y path saneado, con política explícita para query/auth y virtual hosts.
