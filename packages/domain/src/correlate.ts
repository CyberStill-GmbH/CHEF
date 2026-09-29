import type {
  Asset,
  ExposurePath,
  Observation,
  Relationship,
} from './model.js';
import type { IdentityPort } from './ports.js';

/** Pure correlation. Never equate shared addresses, banners, or port numbers with vulnerabilities. */
export function correlate(
  observations: readonly Observation[],
  identity: IdentityPort,
) {
  const assets = new Map<string, Asset>();
  const relationships = new Map<string, Relationship>();
  const ordered = [...observations].sort((a, b) => a.id.localeCompare(b.id));
  function asset(
    kind: Asset['kind'],
    key: string,
    observation: Observation,
  ): Asset {
    const id = identity.id('asset', [kind, key]);
    let item = assets.get(id);
    if (!item) {
      item = {
        id,
        kind,
        key,
        firstSeen: observation.observedAt,
        lastSeen: observation.observedAt,
        observationIds: [],
      };
      assets.set(id, item);
    }
    item.firstSeen =
      item.firstSeen < observation.observedAt
        ? item.firstSeen
        : observation.observedAt;
    item.lastSeen =
      item.lastSeen > observation.observedAt
        ? item.lastSeen
        : observation.observedAt;
    if (!item.observationIds.includes(observation.id))
      item.observationIds.push(observation.id);
    return item;
  }
  for (const observation of ordered) {
    const address = asset('address', observation.address, observation);
    const service = asset(
      'service',
      `${observation.address}|${observation.protocol}|${observation.port}`,
      observation,
    );
    const id = identity.id('relationship', ['exposes', address.id, service.id]);
    let relation = relationships.get(id);
    if (!relation) {
      relation = {
        id,
        from: address.id,
        to: service.id,
        kind: 'exposes',
        assertion: 'observed',
        observationIds: [],
        confidence: observation.confidence,
      };
      relationships.set(id, relation);
    }
    relation.confidence = Math.max(relation.confidence, observation.confidence);
    if (!relation.observationIds.includes(observation.id))
      relation.observationIds.push(observation.id);
  }
  const relations = [...relationships.values()].sort((a, b) =>
    a.id.localeCompare(b.id),
  );
  const paths: ExposurePath[] = relations.map((relation) => ({
    id: identity.id('path', ['open-service/v1', relation.id]),
    assetIds: [relation.from, relation.to],
    relationshipIds: [relation.id],
    assertion: 'inferred',
    ruleId: 'open-service/v1',
    explanation:
      'An imported observation reports an open service. Reachability now and exploitability have not been confirmed.',
    confidence: relation.confidence,
    isConfirmedVulnerability: false,
  }));
  return {
    assets: [...assets.values()].sort((a, b) => a.id.localeCompare(b.id)),
    relationships: relations,
    paths,
  };
}
