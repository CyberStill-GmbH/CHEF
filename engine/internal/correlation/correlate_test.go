package correlation

import (
	"encoding/json"
	"math/rand"
	"os"
	"path/filepath"
	"reflect"
	"testing"
)

// Los fixtures compartidos son la frontera de conformidad TS/Go: el mismo
// correlation-job.json produce el correlation-result.json que emite correlate.ts
// (validado por check:docs en el lado Node).
func readFixture(tb testing.TB, name string) []byte {
	tb.Helper()
	path := filepath.Join("..", "..", "..", "docs", "contracts", "examples", name)
	data, err := os.ReadFile(path)
	if err != nil {
		tb.Fatalf("read fixture %s: %v", name, err)
	}
	return data
}

func loadJob(tb testing.TB, name string) Job {
	tb.Helper()
	var job Job
	if err := json.Unmarshal(readFixture(tb, name), &job); err != nil {
		tb.Fatalf("unmarshal %s: %v", name, err)
	}
	return job
}

func loadResult(tb testing.TB, name string) Result {
	tb.Helper()
	var result Result
	if err := json.Unmarshal(readFixture(tb, name), &result); err != nil {
		tb.Fatalf("unmarshal %s: %v", name, err)
	}
	return result
}

func TestConformanceWithSharedFixture(t *testing.T) {
	got := Correlate(loadJob(t, "correlation-job.json"))
	want := loadResult(t, "correlation-result.json")
	if !reflect.DeepEqual(got, want) {
		gotJSON, _ := json.MarshalIndent(got, "", "  ")
		wantJSON, _ := json.MarshalIndent(want, "", "  ")
		t.Fatalf("output does not match shared fixture.\n got:\n%s\nwant:\n%s", gotJSON, wantJSON)
	}
}

func TestOrderIndependentDeterminism(t *testing.T) {
	job := loadJob(t, "correlation-job.json")
	base := Correlate(job)
	rng := rand.New(rand.NewSource(1))
	for i := 0; i < 25; i++ {
		shuffled := make([]Observation, len(job.Observations))
		copy(shuffled, job.Observations)
		rng.Shuffle(len(shuffled), func(a, b int) { shuffled[a], shuffled[b] = shuffled[b], shuffled[a] })
		reordered := job
		reordered.Observations = shuffled
		if got := Correlate(reordered); !reflect.DeepEqual(got, base) {
			t.Fatalf("result changed with input order at iteration %d", i)
		}
	}
}

func TestNeverConfirmsVulnerability(t *testing.T) {
	got := Correlate(loadJob(t, "correlation-job.json"))
	if len(got.Paths) == 0 {
		t.Fatal("expected at least one path")
	}
	for _, p := range got.Paths {
		if p.IsConfirmedVulnerability {
			t.Fatalf("path %s confirmed a vulnerability", p.ID)
		}
		if p.Status != "inferred" {
			t.Fatalf("path %s has status %q, want inferred", p.ID, p.Status)
		}
	}
}

func TestValidateJobAcceptsPositiveRejectsNegative(t *testing.T) {
	if err := ValidateJob(loadJob(t, "correlation-job.json")); err != nil {
		t.Fatalf("positive job should validate: %v", err)
	}
	var bad Job
	if err := json.Unmarshal(readFixture(t, "correlation-job.invalid.json"), &bad); err != nil {
		t.Fatalf("unmarshal invalid job: %v", err)
	}
	if err := ValidateJob(bad); err == nil {
		t.Fatal("negative job (confidence out of range) should be rejected")
	}
}

func TestValidateResultRejectsConfirmedVulnerability(t *testing.T) {
	if err := ValidateResult(loadResult(t, "correlation-result.json")); err != nil {
		t.Fatalf("positive result should validate: %v", err)
	}
	if err := ValidateResult(loadResult(t, "correlation-result.invalid.json")); err == nil {
		t.Fatal("result claiming a confirmed vulnerability should be rejected")
	}
}
