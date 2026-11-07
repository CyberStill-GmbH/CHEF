package correlation

import (
	"fmt"
	"regexp"
	"time"
)

// Validadores estructurales equivalentes a los JSON Schema
// docs/contracts/correlation-job.schema.json y correlation-result.schema.json.
// Los fixtures negativos compartidos (correlation-*.invalid.json) deben ser
// rechazados aquí igual que por ajv en check:docs.

var (
	reSha256     = regexp.MustCompile(`^[0-9a-f]{64}$`)
	reEvidenceID = regexp.MustCompile(`^evidence:[0-9a-f]{64}$`)
	reObsID      = regexp.MustCompile(`^observation:[0-9a-f]{64}$`)
	reRunID      = regexp.MustCompile(`^run:[0-9a-f]{64}$`)
	reRuleSet    = regexp.MustCompile(`^v[0-9]+$`)
	reAssetID    = regexp.MustCompile(`^asset:[0-9a-f]{64}$`)
	reRelID      = regexp.MustCompile(`^relationship:[0-9a-f]{64}$`)
	rePathID     = regexp.MustCompile(`^path:[0-9a-f]{64}$`)
)

func isProtocol(s string) bool { return s == "tcp" || s == "udp" }
func isSource(s string) bool   { return s == "nmap-xml" || s == "common-crawl-index" }
func isStatus(s string) bool {
	return s == "observed" || s == "inferred" || s == "candidate" || s == "conflicted"
}
func isDateTime(s string) bool { _, err := time.Parse(time.RFC3339, s); return err == nil }
func isPort(p int) bool        { return p >= 0 && p <= 65535 }
func isConfidence(c float64) bool {
	return c >= 0 && c <= 1
}

// ValidateJob verifica un CorrelationJob v0 contra el contrato.
func ValidateJob(job Job) error {
	if job.SchemaVersion != "correlation-job/v0" {
		return fmt.Errorf("schemaVersion must be correlation-job/v0")
	}
	if !reRuleSet.MatchString(job.RuleSetVersion) {
		return fmt.Errorf("ruleSetVersion must match ^v[0-9]+$")
	}
	if !isDateTime(job.AsOf) {
		return fmt.Errorf("asOf must be an RFC3339 date-time")
	}
	if err := validateScope(job.ProjectScope); err != nil {
		return fmt.Errorf("projectScope: %w", err)
	}
	for i, e := range job.Evidence {
		if err := validateEvidence(e); err != nil {
			return fmt.Errorf("evidence[%d]: %w", i, err)
		}
	}
	for i, o := range job.Observations {
		if err := validateObservation(o); err != nil {
			return fmt.Errorf("observations[%d]: %w", i, err)
		}
	}
	return nil
}

func validateScope(s Scope) error {
	if s.ID == "" {
		return fmt.Errorf("id is required")
	}
	if !reSha256.MatchString(s.Digest) {
		return fmt.Errorf("digest must be 64 hex chars")
	}
	for _, p := range s.AllowedPorts {
		if !isPort(p) {
			return fmt.Errorf("allowedPorts out of range")
		}
	}
	for _, proto := range s.AllowedProtocols {
		if !isProtocol(proto) {
			return fmt.Errorf("allowedProtocols must be tcp or udp")
		}
	}
	return nil
}

func validateEvidence(e Evidence) error {
	if !reEvidenceID.MatchString(e.ID) {
		return fmt.Errorf("id must match ^evidence:[0-9a-f]{64}$")
	}
	if !reSha256.MatchString(e.SHA256) {
		return fmt.Errorf("sha256 must be 64 hex chars")
	}
	if !isSource(e.Source) {
		return fmt.Errorf("source must be nmap-xml or common-crawl-index")
	}
	if !isDateTime(e.ImportedAt) {
		return fmt.Errorf("importedAt must be an RFC3339 date-time")
	}
	return nil
}

func validateObservation(o Observation) error {
	if !reObsID.MatchString(o.ID) {
		return fmt.Errorf("id must match ^observation:[0-9a-f]{64}$")
	}
	if o.Address == "" {
		return fmt.Errorf("address is required")
	}
	if !isProtocol(o.Protocol) {
		return fmt.Errorf("protocol must be tcp or udp")
	}
	if !isPort(o.Port) {
		return fmt.Errorf("port out of range")
	}
	if !isDateTime(o.ObservedAt) {
		return fmt.Errorf("observedAt must be an RFC3339 date-time")
	}
	if !reEvidenceID.MatchString(o.EvidenceID) {
		return fmt.Errorf("evidenceId must match ^evidence:[0-9a-f]{64}$")
	}
	if !reRunID.MatchString(o.RunID) {
		return fmt.Errorf("runId must match ^run:[0-9a-f]{64}$")
	}
	if !isSource(o.Source) {
		return fmt.Errorf("source must be nmap-xml or common-crawl-index")
	}
	if o.Fact != "open-port" && o.Fact != "passive-signal" {
		return fmt.Errorf("fact must be open-port or passive-signal")
	}
	if !isConfidence(o.Confidence) {
		return fmt.Errorf("confidence must be between 0 and 1")
	}
	return nil
}

// ValidateResult verifica la salida y, sobre todo, el invariante de seguridad:
// ningún path confirma una vulnerabilidad.
func ValidateResult(r Result) error {
	if r.SchemaVersion != "correlation-result/v0" {
		return fmt.Errorf("schemaVersion must be correlation-result/v0")
	}
	if !reRuleSet.MatchString(r.RuleSetVersion) {
		return fmt.Errorf("ruleSetVersion must match ^v[0-9]+$")
	}
	for i, a := range r.Assets {
		if !reAssetID.MatchString(a.ID) {
			return fmt.Errorf("assets[%d]: id must match ^asset:[0-9a-f]{64}$", i)
		}
		if a.Kind != "address" && a.Kind != "service" {
			return fmt.Errorf("assets[%d]: kind must be address or service", i)
		}
	}
	for i, rel := range r.Relationships {
		if !reRelID.MatchString(rel.ID) {
			return fmt.Errorf("relationships[%d]: id must match ^relationship:[0-9a-f]{64}$", i)
		}
		if rel.Kind != "exposes" {
			return fmt.Errorf("relationships[%d]: kind must be exposes", i)
		}
		if !isStatus(rel.Status) {
			return fmt.Errorf("relationships[%d]: invalid status", i)
		}
		if !isConfidence(rel.Confidence) {
			return fmt.Errorf("relationships[%d]: confidence out of range", i)
		}
	}
	for i, p := range r.Paths {
		if !rePathID.MatchString(p.ID) {
			return fmt.Errorf("paths[%d]: id must match ^path:[0-9a-f]{64}$", i)
		}
		if p.IsConfirmedVulnerability {
			return fmt.Errorf("paths[%d]: isConfirmedVulnerability must be false", i)
		}
		if p.RuleID != "open-service/v1" || p.RuleVersion != "v1" {
			return fmt.Errorf("paths[%d]: unexpected rule", i)
		}
		if !isStatus(p.Status) {
			return fmt.Errorf("paths[%d]: invalid status", i)
		}
		if !isConfidence(p.Confidence) {
			return fmt.Errorf("paths[%d]: confidence out of range", i)
		}
	}
	return nil
}
