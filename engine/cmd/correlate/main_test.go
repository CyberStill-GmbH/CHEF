package main

import (
	"bytes"
	"encoding/json"
	"os"
	"path/filepath"
	"testing"
)

func fixture(t *testing.T, name string) []byte {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("..", "..", "..", "docs", "contracts", "examples", name))
	if err != nil {
		t.Fatal(err)
	}
	return data
}

func TestRunConformance(t *testing.T) {
	var out bytes.Buffer
	input := append(fixture(t, "correlation-job.json"), []byte(" \n\t")...)
	if err := run(bytes.NewReader(input), &out); err != nil {
		t.Fatal(err)
	}
	var got, want any
	if err := json.Unmarshal(out.Bytes(), &got); err != nil {
		t.Fatal(err)
	}
	if err := json.Unmarshal(fixture(t, "correlation-result.json"), &want); err != nil {
		t.Fatal(err)
	}
	gotJSON, _ := json.Marshal(got)
	wantJSON, _ := json.Marshal(want)
	if !bytes.Equal(gotJSON, wantJSON) {
		t.Fatal("executable output differs from shared result fixture")
	}
}

func TestRunRejectsInvalidInputWithoutOutput(t *testing.T) {
	valid := fixture(t, "correlation-job.json")
	for name, input := range map[string][]byte{
		"empty":           nil,
		"malformed":       []byte("{"),
		"invalid job":     fixture(t, "correlation-job.invalid.json"),
		"second document": append(bytes.Clone(valid), []byte("\n{}")...),
		"trailing data":   append(bytes.Clone(valid), []byte("secret-payload")...),
		"oversized":       bytes.Repeat([]byte(" "), maxInputBytes+1),
	} {
		t.Run(name, func(t *testing.T) {
			var out bytes.Buffer
			err := run(bytes.NewReader(input), &out)
			if err == nil {
				t.Fatal("invalid input was accepted")
			}
			if out.Len() != 0 {
				t.Fatal("invalid input published output")
			}
			if bytes.Contains([]byte(err.Error()), []byte("secret-payload")) {
				t.Fatal("diagnostic contains input payload")
			}
		})
	}
}
