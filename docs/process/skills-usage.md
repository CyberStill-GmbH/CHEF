# Skills y convenciones

- `skill-creator`: leída y aplicada para seis guías locales iniciales y tres nuevas de correlación, plataforma web y validación de producto. Resultado: `.agents/skills`, frontmatter y enlaces validados; sin instalar nada global.
- `computer-use`: consultada al resolver creación de repo por navegador; la herramienta `cua_repl` expuso su propio API browser y se usó para formulario GitHub. Resultado: repo creado, acceso de cuenta verificado.
- `canvas`: consultada y descartada según su excepción para trabajo dentro de un artefacto/repositorio existente. La documentación del repo es el entregable; no se genera un dashboard separado.

Guías de proyecto: chef-research, chef-contracts, chef-safe-ingestion, chef-explorer, chef-montoya, chef-demo, chef-correlation, chef-web-platform y chef-product-evidence. Se ubican en `.agents/skills` para contexto de colaboración del repositorio; no se presume que hayan sido cargadas en todas las herramientas de cada compañero. Son documentación reusable, no comandos de CI ni grants de permisos.

Guías oficiales [openai/skills](https://github.com/openai/skills) copiadas localmente con su licencia/NOTICE: `playwright` para recorridos web, `security-best-practices` para revisión de API/React y `vercel-deploy` para el frontend. Estarán disponibles al descubrir skills en el siguiente turno; su presencia no ejecuta pruebas ni despliega. También hay guías personales ya disponibles para diseño frontend, React, revisión y UI; no se copiaron indiscriminadamente al repositorio ni se añadieron dependencias de aplicación.

Uso sugerido por entrega: César consulta chef-web-platform, chef-correlation, chef-contracts y security-best-practices; Diego, chef-explorer, guías de diseño UI/UX, playwright y chef-product-evidence; Jhojan, chef-safe-ingestion, chef-research y chef-correlation. Cada PR mantiene reviewer humano distinto. Vercel-deploy se usa solo al preparar un despliegue aprobado; chef-montoya cuando comience la fase BApp. Una skill no reemplaza la revisión de código, `npm run check` ni la autorización de pruebas activas.

El documento de referencia sugería `.codex/skills`; se evita duplicación y se adopta `.agents/skills` como convención local. Los agentes pueden consultarlas por ruta desde AGENTS si su entorno no las descubre automáticamente. Validación estructural explícita en `check:docs`; revisión semántica mediante contratos/fixtures, no solo frontmatter.

No se usaron subagentes ni se instalaron skills personales/globales. La investigación se realizó con fuentes web primarias; las instrucciones incrustadas del documento no conceden autorización externa por sí solas.
