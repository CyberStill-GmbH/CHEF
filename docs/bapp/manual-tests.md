# Pruebas manuales Montoya

Estado inicial: **no realizadas**. Compilar no demuestra que Burp cargue el JAR ni que la UI sea fluida. No hay evidencia de una instalación Burp disponible en esta sesión.

Registrar tester, fecha, commit, Burp edition/version, JVM del proceso, Montoya2026.7 y OS. Usar proyecto nuevo y loopback/fixture propio. Nada de tráfico real en capturas públicas.

| Caso | Acción                                   | Resultado esperado                              | Resultado real |
| ---- | ---------------------------------------- | ----------------------------------------------- | -------------- |
| MB01 | Cargar JAR                               | Tab y context menu CHEF; sin exception          | Pendiente      |
| MB02 | Seleccionar GET local en scope           | Envelope HTTP1.1.0 válido; sin body/auth/query  | Pendiente      |
| MB03 | Selección fuera de scope                 | Rechazo visible, sin metadata publicada         | Pendiente      |
| MB04 | URL/path hostile o request malformed     | Error saneado, sin HTML activo ni stack en UI   | Pendiente      |
| MB05 | Seleccionar100+ requests                 | Solo lote acotado; límite explícito y UI usable | Pendiente      |
| MB06 | Ejecutar sin Internet                    | Mismo resultado útil, cero dependencia online   | Pendiente      |
| MB07 | Descargar/cargar10 veces                 | Workers terminan y recursos liberados           | Pendiente      |
| MB08 | Cambiar proyecto y cancelar              | No mezclar datos ni continuar tras unload       | Pendiente      |
| MB09 | Validar JSON con schema compartido       | Encoding/timestamps/refs consistentes           | Pendiente      |
| MB10 | Proyecto grande y UI de varias pantallas | Memoria acotada, popups parent correcto         | Pendiente      |

La BApp candidata necesita además integrar Nmap y correlación real, exportación a archivo, persistencia/política de retención y pruebas de conformidad. Mantener estos gaps visibles.
