// Package correlation implementa el motor de correlación Go del ADR 0011.
//
// Recibe un CorrelationJob v0 (observaciones ya canónicas, scope explícito,
// reloj fijo y versión de reglas), aplica la regla open-service/v1 y devuelve
// un CorrelationResult v0 determinista. No accede a red, disco, base ni
// credenciales: solo transforma los datos que recibe.
//
// Los tipos y los IDs se corresponden con la implementación TypeScript aceptada
// (packages/domain/src/model.ts y correlate.ts), que es la referencia de
// conformidad; los fixtures compartidos en docs/contracts/examples/ lo fijan.
package correlation

// Job es la entrada CorrelationJob v0.
type Job struct {
	SchemaVersion  string        `json:"schemaVersion"`
	RuleSetVersion string        `json:"ruleSetVersion"`
	AsOf           string        `json:"asOf"`
	ProjectScope   Scope         `json:"projectScope"`
	Evidence       []Evidence    `json:"evidence"`
	Observations   []Observation `json:"observations"`
}

// Scope es el scope autorizado del proyecto para este trabajo.
type Scope struct {
	ID                string   `json:"id"`
	Digest            string   `json:"digest"`
	AllowedAddresses  []string `json:"allowedAddresses"`
	ExcludedAddresses []string `json:"excludedAddresses"`
	AllowedPorts      []int    `json:"allowedPorts"`
	AllowedProtocols  []string `json:"allowedProtocols"`
}

// Evidence referencia la fuente importada por digest, sin el contenido.
type Evidence struct {
	ID         string `json:"id"`
	SHA256     string `json:"sha256"`
	Source     string `json:"source"`
	Locator    string `json:"locator"`
	ImportedAt string `json:"importedAt"`
}

// Observation es una observación canónica ya validada por Node.
type Observation struct {
	ID         string  `json:"id"`
	Address    string  `json:"address"`
	Protocol   string  `json:"protocol"`
	Port       int     `json:"port"`
	ObservedAt string  `json:"observedAt"`
	Locator    string  `json:"locator"`
	EvidenceID string  `json:"evidenceId"`
	RunID      string  `json:"runId"`
	Source     string  `json:"source"`
	Fact       string  `json:"fact"`
	Confidence float64 `json:"confidence"`
}

// Result es la salida CorrelationResult v0.
type Result struct {
	SchemaVersion  string         `json:"schemaVersion"`
	RuleSetVersion string         `json:"ruleSetVersion"`
	AsOf           string         `json:"asOf"`
	Assets         []Asset        `json:"assets"`
	Relationships  []Relationship `json:"relationships"`
	Paths          []Path         `json:"paths"`
	Metrics        Metrics        `json:"metrics"`
}

// Asset es una dirección o un servicio deduplicado.
type Asset struct {
	ID             string   `json:"id"`
	Kind           string   `json:"kind"`
	Key            string   `json:"key"`
	FirstSeen      string   `json:"firstSeen"`
	LastSeen       string   `json:"lastSeen"`
	ObservationIDs []string `json:"observationIds"`
}

// Relationship es una exposición observada entre dirección y servicio.
type Relationship struct {
	ID             string   `json:"id"`
	From           string   `json:"from"`
	To             string   `json:"to"`
	Kind           string   `json:"kind"`
	Status         string   `json:"status"`
	ObservationIDs []string `json:"observationIds"`
	Confidence     float64  `json:"confidence"`
}

// Path es una inferencia de superficie. isConfirmedVulnerability es siempre
// false: la regla nunca confirma una vulnerabilidad.
type Path struct {
	ID                       string   `json:"id"`
	AssetIDs                 []string `json:"assetIds"`
	RelationshipIDs          []string `json:"relationshipIds"`
	RuleID                   string   `json:"ruleId"`
	RuleVersion              string   `json:"ruleVersion"`
	Status                   string   `json:"status"`
	Explanation              string   `json:"explanation"`
	Confidence               float64  `json:"confidence"`
	IsConfirmedVulnerability bool     `json:"isConfirmedVulnerability"`
}

// Metrics resume la deduplicación del trabajo.
type Metrics struct {
	ObservationsIn        int `json:"observationsIn"`
	AssetsOut             int `json:"assetsOut"`
	RelationshipsOut      int `json:"relationshipsOut"`
	PathsOut              int `json:"pathsOut"`
	DuplicateObservations int `json:"duplicateObservations"`
}
