# Licencias y atribución

Código y documentación originales CHEF: MIT, elegido para facilitar mantenimiento/contribución y distribución pública. El equipo debe confirmar titularidad y permiso de cada aporte antes de la postulación; esta licencia no concede permiso para reutilizar código ajeno ni para someter trabajo sin consentimiento de autores.

Dependencias directas runtime: fast-xml-parser, Ajv y ajv-formats (MIT según sus package metadata en las versiones fijadas). Herramientas TS/ESLint/Prettier/Mermaid/YAML y transitivas tienen sus propios notices. Mantener inventario obtenido del lock/install con [script](../../scripts/dependency-licenses.mjs); revisar cualquier valor desconocido antes de distribuir.

Montoya API usa términos propios de PortSwigger: [licencia oficial](https://github.com/PortSwigger/burp-extensions-montoya-api/blob/main/LICENSE). Se declara provided/compileOnly, sin incluir API ni ejemplos copiados en JAR. El spike está escrito para CHEF; no se presupone que los ejemplos oficiales sean MIT.

Nmap tiene [NPSL/condiciones propias](https://nmap.org/book/man-legal.html). CHEF importa XML y no incorpora código/binario/DTD/Npcap ni ejecuta Nmap. Si se planea empaquetarlo, resolver condiciones antes del cambio. Amass/SpiderFoot/Uncover se comparan; no se redistribuyen sus implementaciones.

Cytoscape core MIT y Sigma son candidatos UI; todavía no están instalados para la app. Revalidar versiones/plugins/licencias al elegirlos. No usar logos de terceros como si hubiera afiliación. Para BApp Store verificar EULA y consentimiento de todos los autores según [portal oficial](https://github.com/PortSwigger/extension-portal).
