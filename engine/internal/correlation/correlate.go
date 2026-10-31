package correlation

import (
	"fmt"
	"sort"
)

const openServiceExplanation = "An imported observation reports an open service. Reachability now and exploitability have not been confirmed."

// Correlate aplica la regla open-service/v1 de forma determinista, reproduciendo
// packages/domain/src/correlate.ts: deduplica direcciones y servicios, deriva
// relaciones "exposes" y una ruta inferida por relación. Nunca equipara una
// dirección, un banner o un puerto compartido con una vulnerabilidad.
func Correlate(job Job) Result {
	ordered := make([]Observation, len(job.Observations))
	copy(ordered, job.Observations)
	sort.SliceStable(ordered, func(i, j int) bool { return ordered[i].ID < ordered[j].ID })

	assets := map[string]*Asset{}
	relationships := map[string]*Relationship{}
	seenServiceKeys := map[string]bool{}
	duplicates := 0

	ensureAsset := func(kind, key string, o Observation) *Asset {
		assetID := id("asset", []string{kind, key})
		item := assets[assetID]
		if item == nil {
			item = &Asset{
				ID:             assetID,
				Kind:           kind,
				Key:            key,
				FirstSeen:      o.ObservedAt,
				LastSeen:       o.ObservedAt,
				ObservationIDs: []string{},
			}
			assets[assetID] = item
		}
		if o.ObservedAt < item.FirstSeen {
			item.FirstSeen = o.ObservedAt
		}
		if o.ObservedAt > item.LastSeen {
			item.LastSeen = o.ObservedAt
		}
		if !contains(item.ObservationIDs, o.ID) {
			item.ObservationIDs = append(item.ObservationIDs, o.ID)
		}
		return item
	}

	for _, o := range ordered {
		address := ensureAsset("address", o.Address, o)
		serviceKey := fmt.Sprintf("%s|%s|%d", o.Address, o.Protocol, o.Port)
		if seenServiceKeys[serviceKey] {
			duplicates++
		} else {
			seenServiceKeys[serviceKey] = true
		}
		service := ensureAsset("service", serviceKey, o)

		relID := id("relationship", []string{"exposes", address.ID, service.ID})
		relation := relationships[relID]
		if relation == nil {
			relation = &Relationship{
				ID:             relID,
				From:           address.ID,
				To:             service.ID,
				Kind:           "exposes",
				Status:         "observed",
				ObservationIDs: []string{},
				Confidence:     o.Confidence,
			}
			relationships[relID] = relation
		}
		if o.Confidence > relation.Confidence {
			relation.Confidence = o.Confidence
		}
		if !contains(relation.ObservationIDs, o.ID) {
			relation.ObservationIDs = append(relation.ObservationIDs, o.ID)
		}
	}

	relations := make([]Relationship, 0, len(relationships))
	for _, r := range relationships {
		relations = append(relations, *r)
	}
	sort.SliceStable(relations, func(i, j int) bool { return relations[i].ID < relations[j].ID })

	paths := make([]Path, 0, len(relations))
	for _, r := range relations {
		paths = append(paths, Path{
			ID:                       id("path", []string{"open-service/v1", r.ID}),
			AssetIDs:                 []string{r.From, r.To},
			RelationshipIDs:          []string{r.ID},
			RuleID:                   "open-service/v1",
			RuleVersion:              "v1",
			Status:                   "inferred",
			Explanation:              openServiceExplanation,
			Confidence:               r.Confidence,
			IsConfirmedVulnerability: false,
		})
	}

	assetList := make([]Asset, 0, len(assets))
	for _, a := range assets {
		assetList = append(assetList, *a)
	}
	sort.SliceStable(assetList, func(i, j int) bool { return assetList[i].ID < assetList[j].ID })

	return Result{
		SchemaVersion:  "correlation-result/v0",
		RuleSetVersion: job.RuleSetVersion,
		AsOf:           job.AsOf,
		Assets:         assetList,
		Relationships:  relations,
		Paths:          paths,
		Metrics: Metrics{
			ObservationsIn:        len(job.Observations),
			AssetsOut:             len(assetList),
			RelationshipsOut:      len(relations),
			PathsOut:              len(paths),
			DuplicateObservations: duplicates,
		},
	}
}

func contains(items []string, value string) bool {
	for _, item := range items {
		if item == value {
			return true
		}
	}
	return false
}
