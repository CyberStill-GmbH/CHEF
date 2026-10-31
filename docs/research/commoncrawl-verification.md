# Verificación Common Crawl para CHEF

**Fecha:** 2026-10-24  
**Responsable:** Jhojan  
**Gate:** 1 (05-10-2026) - Decisión Go/No-Go para adapter

---

## 1. Términos de Uso - Análisis

**Fuente:** https://commoncrawl.org/terms-of-use

| Aspecto                           | Estado         | Detalle                                                                                       |
| --------------------------------- | -------------- | --------------------------------------------------------------------------------------------- |
| **Consulta automática permitida** | ✅ Sí          | "You may use the Common Crawl dataset for research, analysis, and educational purposes"       |
| **Rate limiting documentado**     | ⚠️ Parcial     | No hay cuota pública explícita; se recomienda backoff exponencial y User-Agent identificativo |
| **API Key / Registro requerido**  | ❌ No          | Índice CDX público sin autenticación                                                          |
| **Redistribución de datos**       | ⚠️ Restringida | "You may not redistribute the raw dataset" - solo resultados agregados/derivados              |
| **Atribución requerida**          | ✅ Sí          | "Common Crawl" + URL del dataset usado                                                        |
| **Uso comercial**                 | ⚠️ Revisar     | "Non-commercial research" - CHEF es herramienta de seguridad para pentesting autorizado       |

**Conclusión:** Uso **permitido para investigación/no-comercial** con atribución. Para uso en pentesting comercial, requiere revisión legal adicional.

---

## 2. Acceso Técnico - Verificación

### Índice CDX (API pública)

```
Endpoint: https://index.commoncrawl.org/
Formatos: CDX (JSON lines), CDXJ
Parámetros: url, matchType, output, filter, limit, offset
```

**Ejemplo consulta:**

```bash
curl "https://index.commoncrawl.org/CC-MAIN-2024-51-index?url=example.com&output=json&limit=10"
```

### Datasets disponibles (2024-2025)

| Dataset         | Fecha    | Tamaño | Cobertura |
| --------------- | -------- | ------ | --------- |
| CC-MAIN-2024-51 | Dic 2024 | ~400TB | Global    |
| CC-MAIN-2024-46 | Nov 2024 | ~380TB | Global    |
| CC-MAIN-2024-41 | Oct 2024 | ~360TB | Global    |

**Actualización:** Mensual aprox.

---

## 3. Dominio/IP de Prueba AUTORIZADO

**REQUISITO CRÍTICO:** Para validar adapter real se necesita:

- [ ] Dominio/IP con **autorización escrita** del cliente
- [ ] Nmap XML correspondiente (mismo scope)
- [ ] Resultados en Common Crawl verificables

**Estado actual:** ❌ **NO DISPONIBLE** - Pendiente confirmación del PO (César)

**Fallback documentado:** Si no hay autorización real → fixture sintético etiquetado + gap documentado en ADR.

---

## 4. Cuotas y Límites Operativos

| Límite                | Valor                                 | Fuente                                    |
| --------------------- | ------------------------------------- | ----------------------------------------- |
| Requests concurrentes | ≤ 8                                   | Recomendación general CDX                 |
| Timeout sugerido      | 30s                                   | Empírico                                  |
| Backoff en 429        | Exponencial (1s, 2s, 4s, 8s, max 60s) | Best practice                             |
| Paginación            | `offset` + `limit` (max 10000)        | API CDX                                   |
| TTL cache             | 24h                                   | Recomendado (datos históricos inmutables) |

---

## 5. Decisión Gate 1

| Criterio                           | Cumple | Evidencia                  |
| ---------------------------------- | ------ | -------------------------- |
| Términos permiten uso no-comercial | ✅     | Sección 1                  |
| Acceso técnico verificado          | ✅     | Sección 2                  |
| Dominio de prueba autorizado       | ❌     | Sección 3 - **BLOQUEANTE** |
| Cuotas documentadas                | ✅     | Sección 4                  |
| Fixture sintético preparado        | ⏳     | Pendiente creación         |

### **DECISIÓN: CONDITIONAL GO**

- ✅ **Adapter se implementa** (arquitectura lista)
- ✅ **Fixtures sintéticos** para tests/CI
- ⚠️ **Integración real** bloqueada hasta dominio autorizado
- 📝 **Gap registrado** en ADR propuesta

---

## 6. Próximos Pasos

1. **Crear adapter** `packages/adapters/src/commoncrawl.ts` (implementa `ImportParser`)
2. **Crear fixtures** `fixtures/commoncrawl/` (positivo, negativo, truncado, vacío, rate-limited)
3. **Añadir tests** en `tests/jhojan-ingestion.test.ts`
4. **Registrar ADR** con decisión y gap de dominio autorizado
5. **Esperar confirmación PO** para dominio real → prueba de integración completa
