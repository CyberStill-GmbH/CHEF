# Desarrollo local

Node 24 LTS; versiones exactas en `package-lock.json`. Ejecuta desde la raíz del repositorio. La CLI usa rutas relativas para el schema y los fixtures, por lo que no se distribuye aún como paquete global.

```sh
npm ci
npm run check
npm run demo
npm run benchmark
```

`check` incluye ESLint, TypeScript estricto, pruebas unitarias/integración/CLI, enlaces locales, frontmatter, Mermaid, ejemplos de contratos, reglas de dependencia y Prettier. El JSON de demo debe extraerse usando directamente `node dist/apps/cli/src/main.js`, porque npm agrega mensajes a stdout.

En PowerShell usa `npm.cmd` si la política local bloquea `npm.ps1`. No necesitas cambiar esa política ni instalar herramientas globales. `dist` y `artifacts` están ignorados. Para registrar resultados localmente: crea `artifacts` y redirige la salida del comando que corresponda.

Java se desarrolla en `extensions/burp`, con JDK 21 y Maven 3.9.x. El JRE 8 encontrado inicialmente no sirve para compilar el spike. El build no requiere Burp; cargar y probar el JAR sí requiere una instalación de Burp y un laboratorio local. La versión mínima de Burp se determina mediante pruebas, no se inventa desde la versión del API.

Flujo recomendado: fixture mínimo → cambio de contrato si procede → implementación → pruebas → PR → revisión de compañero. Para depurar alcance usa únicamente loopback o laboratorio autorizado. No metas exportaciones reales, `.env`, capturas de tráfico o logs en Git.
