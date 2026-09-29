# Seguridad

CHEF está en desarrollo; todavía no tiene una versión estable soportada. El único código respaldado por CI es el contenido actual de `main`.

Para problemas que exponen secretos o permiten ejecución arbitraria, utiliza GitHub **Security → Report a vulnerability** cuando la función esté habilitada. Si no está disponible, abre una issue solicitando un canal privado, sin payloads explotables ni datos sensibles. No hay SLA de respuesta confirmado para este equipo estudiantil.

Las issues públicas deben contener únicamente fixtures sintéticos. El programa no escanea, ejecuta shell, resuelve DNS ni envía telemetría en el incremento actual. Consulta el [modelo de amenazas](docs/security/threat-model.md) y la [política de datos](docs/security/data-policy.md).
