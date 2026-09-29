---
name: chef-montoya
description: Revisar el adaptador Java/Montoya y evidencia de preparación BApp de CHEF.
---

# chef-montoya

Usar al cambiar extensions/burp o preparar una revisión BApp Store.

Leer ADR0006, docs/bapp/acceptance.md y manual-tests.md. Verificar API oficial para versión fijada; compilar no equivale a cargar en Burp. Mantener utilidad autónoma y scope de proyecto, sin dependencias de servicio obligatorias. Batches/background bounded, Swing en EDT, catch/log y unload limpio. No retener objetos de tráfico a largo plazo. Validar envelope y golden fixtures. Registrar pruebas manuales pendientes; no enviar submission salvo autorización posterior explícita.

Las rutas mencionadas son relativas a la raíz del repositorio CHEF. Ejecutar npm run check para los cambios pertinentes y registrar resultados reales.
