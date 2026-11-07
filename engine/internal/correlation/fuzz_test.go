package correlation

import (
	"encoding/json"
	"reflect"
	"testing"
)

// FuzzValidateJob comprueba que el validador nunca entra en pánico ante
// cualquier JSON que decodifique en un Job.
func FuzzValidateJob(f *testing.F) {
	f.Add(readFixture(f, "correlation-job.json"))
	f.Add(readFixture(f, "correlation-job.invalid.json"))
	f.Add([]byte("{}"))
	f.Fuzz(func(t *testing.T, data []byte) {
		var job Job
		if json.Unmarshal(data, &job) != nil {
			return
		}
		_ = ValidateJob(job)
	})
}

// FuzzCorrelate comprueba, para cualquier job válido, los invariantes del ADR:
// nunca confirma una vulnerabilidad, el resultado respeta el contrato y la
// correlación es determinista.
func FuzzCorrelate(f *testing.F) {
	f.Add(readFixture(f, "correlation-job.json"))
	f.Fuzz(func(t *testing.T, data []byte) {
		var job Job
		if json.Unmarshal(data, &job) != nil {
			return
		}
		if ValidateJob(job) != nil {
			return
		}
		result := Correlate(job)
		if err := ValidateResult(result); err != nil {
			t.Fatalf("valid job produced an invalid result: %v", err)
		}
		for _, p := range result.Paths {
			if p.IsConfirmedVulnerability {
				t.Fatal("a path confirmed a vulnerability")
			}
		}
		if !reflect.DeepEqual(Correlate(job), result) {
			t.Fatal("correlation is not deterministic")
		}
	})
}
