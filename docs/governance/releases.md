# Releases y mantenimiento

Estado0.1.0: preparación/core offline/spike, no release estable. No publicar binario como extensión aceptada. Demo tag propuesto `v0.1.0-demo` después de ensayos reales y review del equipo; el bootstrap no crea ese tag.

Cada release incluirá changelog, schema/rules/parser version, commit, build reproducible, SHA-256 de artifact, licencias/notices, guía de uso y límites. Construir de checkout limpio usando lockfile y JDK/API fijados. Mantener `project.build.outputTimestamp` para JAR reproducible; comparar dos builds limpios.

Compatibilidad wire: major cuando cambia identidad/semántica; minor para extensión compatible solo si reader soporta explícitamente la versión. Actualmente Nmap snapshot1.0.0 y HTTP envelope1.1.0 propuesto son contratos diferentes con schemas distintos.

Dependencias: Dependabot npm/Maven/Actions semanal; revisar alertas, actualización mínima, tests/regresión y licenses. No hardcodear código de severidad como hecho de vigencia permanente. Vulnerabilidad urgente: issue privada, repro sintético, fix/test y comunicación con alcance/impacto verificados. No hay SLA comercial.
