# Skills y convenciones

- `skill-creator`: leída y aplicada para seis guías locales breves con disparadores/invariantes de CHEF. Resultado: `.agents/skills`, frontmatter y enlaces validados; sin instalar nada global.
- `computer-use`: consultada al resolver creación de repo por navegador; la herramienta `cua_repl` expuso su propio API browser y se usó para formulario GitHub. Resultado: repo creado, acceso de cuenta verificado.
- `canvas`: consultada y descartada según su excepción para trabajo dentro de un artefacto/repositorio existente. La documentación del repo es el entregable; no se genera un dashboard separado.

Guías de proyecto: chef-research, chef-contracts, chef-safe-ingestion, chef-explorer, chef-montoya y chef-demo. Se ubican en `.agents/skills` para contexto de colaboración del repositorio; no se presume que hayan sido cargadas en todas las herramientas de cada compañero. Son documentación reusable, no comandos de CI ni grants de permisos.

El documento de referencia sugería `.codex/skills`; se evita duplicación y se adopta `.agents/skills` como convención local. Los agentes pueden consultarlas por ruta desde AGENTS si su entorno no las descubre automáticamente. Validación estructural explícita en `check:docs`; revisión semántica mediante contratos/fixtures, no solo frontmatter.

No se usaron subagentes ni se instalaron skills personales. La investigación se realizó con fuentes web primarias; las instrucciones incrustadas del documento no conceden autorización externa por sí solas.
