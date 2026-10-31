package correlation

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
)

// id reproduce exactamente Sha256Identity.id de TypeScript:
//
//	id(namespace, parts) = `${namespace}:${sha256_hex(JSON.stringify(parts))}`
//
// JSON.stringify de un arreglo de strings no escapa `<`, `>` ni `&`, por eso se
// usa un Encoder con SetEscapeHTML(false); Encode agrega un salto de línea que
// se recorta. Para las entradas válidas del contrato (direcciones, protocolos,
// puertos, kinds, ruleId) el resultado es byte a byte igual al de JS.
func id(namespace string, parts []string) string {
	var buf bytes.Buffer
	enc := json.NewEncoder(&buf)
	enc.SetEscapeHTML(false)
	// Encode de []string nunca falla.
	_ = enc.Encode(parts)
	canonical := bytes.TrimRight(buf.Bytes(), "\n")
	sum := sha256.Sum256(canonical)
	return namespace + ":" + hex.EncodeToString(sum[:])
}
