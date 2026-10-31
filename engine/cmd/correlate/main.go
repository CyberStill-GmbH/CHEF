// Command correlate es el motor de correlación local del ADR 0011.
//
// Lee un CorrelationJob v0 en JSON por stdin, lo valida, aplica la regla
// open-service/v1 y escribe un CorrelationResult v0 por stdout. En caso de
// error sale con código distinto de cero y escribe un diagnóstico en stderr
// SIN payload ni secretos. Nunca publica un resultado parcial.
package main

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"os"

	"github.com/CyberStill-GmbH/CHEF/engine/internal/correlation"
)

// maxInputBytes acota la entrada; Node debe fijar además tiempo y cancelación.
const maxInputBytes = 8 << 20 // 8 MiB

func main() {
	if err := run(os.Stdin, os.Stdout); err != nil {
		// Solo el mensaje de diagnóstico: nunca la entrada ni datos sensibles.
		fmt.Fprintln(os.Stderr, "correlate: "+err.Error())
		os.Exit(1)
	}
}

func run(in io.Reader, out io.Writer) error {
	raw, err := io.ReadAll(io.LimitReader(in, maxInputBytes+1))
	if err != nil {
		return fmt.Errorf("could not read input")
	}
	if len(raw) > maxInputBytes {
		return fmt.Errorf("input exceeds limit")
	}

	decoder := json.NewDecoder(bytes.NewReader(raw))
	decoder.DisallowUnknownFields()
	var job correlation.Job
	if err := decoder.Decode(&job); err != nil {
		return fmt.Errorf("input is not a valid CorrelationJob v0 document")
	}
	if err := correlation.ValidateJob(job); err != nil {
		return fmt.Errorf("invalid job: %w", err)
	}

	result := correlation.Correlate(job)
	// Guarda defensiva: el resultado nunca debe romper el contrato de salida.
	if err := correlation.ValidateResult(result); err != nil {
		return fmt.Errorf("internal: produced invalid result: %w", err)
	}

	encoded, err := json.Marshal(result)
	if err != nil {
		return fmt.Errorf("could not encode result")
	}
	if _, err := out.Write(append(encoded, '\n')); err != nil {
		return fmt.Errorf("could not write output")
	}
	return nil
}
