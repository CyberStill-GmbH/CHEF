import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { ImportEvidence } from '../packages/application/src/import-evidence.js';
import { CommonCrawlIndexParser } from '../packages/adapters/src/commoncrawl.js';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../packages/adapters/src/scope.js';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
import { loadContracts } from '../packages/adapters/src/contracts.js';

const SAMPLES = 100;
const WARMUP = 10;
const FIXED_CLOCK = { now: () => '2026-09-29T00:00:00.000Z' };

const contracts = loadContracts('docs/contracts/snapshot.schema.json');
const policy = contracts.scope(
  JSON.parse(readFileSync('fixtures/scope/lab.json', 'utf8')),
);
const bytes = readFileSync('fixtures/nmap/lab.xml');
const labels: {
  expectedOpenObservations: number;
  expectedAssetKeys: string[];
  positivePairs: number[][];
  negativePairs: number[][];
} = JSON.parse(readFileSync('fixtures/benchmark/labels.json', 'utf8'));
const parser = new NmapXmlParser();
const pipeline = new ImportEvidence(
  parser,
  new ExactScopeGuard(),
  new Sha256Identity(),
  FIXED_CLOCK,
);
const result = pipeline.execute(bytes, policy);
contracts.snapshot(result);
assert.equal(result.observations.length, labels.expectedOpenObservations);
assert.deepEqual(
  result.assets.map((item) => item.key).sort(),
  [...labels.expectedAssetKeys].sort(),
);
assert.ok(result.observations.every((item) => item.fact === 'open-port'));
const inputs = parser.parse(bytes, policy.maxObservations);
const same = (pair: number[]) => {
  const first = inputs[pair[0] ?? -1];
  const second = inputs[pair[1] ?? -1];
  assert.ok(first && second);
  return (
    first.address === second.address &&
    first.port === second.port &&
    first.protocol === second.protocol
  );
};
const truePositive = labels.positivePairs.filter(same).length;
const falsePositive = labels.negativePairs.filter(same).length;
const observationIds = new Set(result.observations.map((item) => item.id));
const evidenceIds = new Set(result.evidence.map((item) => item.id));
const entities = [...result.assets, ...result.relationships];
const traceable = entities.filter(
  (item) =>
    item.observationIds.length &&
    item.observationIds.every(
      (id) =>
        observationIds.has(id) &&
        result.observations.some(
          (observation) =>
            observation.id === id && evidenceIds.has(observation.evidenceId),
        ),
    ),
).length;
for (let i = 0; i < WARMUP; i++) pipeline.execute(bytes, policy);
const samples: number[] = [];
for (let i = 0; i < SAMPLES; i++) {
  const start = performance.now();
  pipeline.execute(bytes, policy);
  samples.push(performance.now() - start);
}
samples.sort((a, b) => a - b);
assert.deepEqual(result, pipeline.execute(bytes, policy));
console.log(
  JSON.stringify(
    {
      dataset: 'synthetic-lab/v1',
      source: 'nmap-xml',
      node: process.version,
      platform: process.platform,
      inputBytes: bytes.length,
      samples: samples.length,
      p50Ms: samples[49],
      p95Ms: samples[94],
      baselineUncorrelatedServices: inputs.length,
      correlatedServices: result.assets.filter(
        (item) => item.kind === 'service',
      ).length,
      truePositive,
      falsePositive,
      precision: truePositive / (truePositive + falsePositive),
      recall: truePositive / labels.positivePairs.length,
      traceableEntityRatio: traceable / entities.length,
      repeatableWithFixedClock: true,
      limitation:
        'Three synthetic observations, one positive pair, two negative pairs. Not evidence of real-world detection accuracy or UI throughput.',
    },
    null,
    2,
  ),
);

// Second source: the same pipeline over a synthetic Common Crawl CDX sample.
// It proves the adapter runs end to end and that a passive URL is never labelled
// as an open port; it is not a measurement of live Common Crawl latency.
const crawlPolicy = contracts.scope({
  schemaVersion: '1.0.0',
  id: 'bench-commoncrawl-v1',
  allowedAddresses: ['93.184.216.34', '151.101.1.140', '151.101.65.140'],
  excludedAddresses: [],
  allowedPorts: [80, 443],
  allowedProtocols: ['tcp'],
  maxInputBytes: 2097152,
  maxObservations: 10000,
});
const crawlBytes = readFileSync('fixtures/commoncrawl/positive.jsonl');
const crawlParser = new CommonCrawlIndexParser(
  crawlPolicy.allowedAddresses,
  crawlPolicy.allowedPorts,
  crawlPolicy.allowedProtocols,
);
const crawlPipeline = new ImportEvidence(
  crawlParser,
  new ExactScopeGuard(),
  new Sha256Identity(),
  FIXED_CLOCK,
);
const crawlResult = crawlPipeline.execute(crawlBytes, crawlPolicy);
contracts.snapshot(crawlResult);
assert.ok(crawlResult.observations.length > 0);
assert.ok(
  crawlResult.observations.every((item) => item.fact === 'passive-signal'),
);
assert.ok(
  crawlResult.observations.every(
    (item) => item.source === 'common-crawl-index',
  ),
);
for (let i = 0; i < WARMUP; i++) crawlPipeline.execute(crawlBytes, crawlPolicy);
const crawlSamples: number[] = [];
for (let i = 0; i < SAMPLES; i++) {
  const start = performance.now();
  crawlPipeline.execute(crawlBytes, crawlPolicy);
  crawlSamples.push(performance.now() - start);
}
crawlSamples.sort((a, b) => a - b);
const crawlEntities = [...crawlResult.assets, ...crawlResult.relationships];
const crawlObservationIds = new Set(
  crawlResult.observations.map((item) => item.id),
);
const crawlEvidenceIds = new Set(crawlResult.evidence.map((item) => item.id));
const crawlTraceable = crawlEntities.filter(
  (item) =>
    item.observationIds.length &&
    item.observationIds.every(
      (id) =>
        crawlObservationIds.has(id) &&
        crawlResult.observations.some(
          (observation) =>
            observation.id === id &&
            crawlEvidenceIds.has(observation.evidenceId),
        ),
    ),
).length;
console.log(
  JSON.stringify(
    {
      dataset: 'synthetic-commoncrawl/v1',
      source: 'common-crawl-index',
      node: process.version,
      platform: process.platform,
      inputBytes: crawlBytes.length,
      samples: crawlSamples.length,
      p50Ms: crawlSamples[49],
      p95Ms: crawlSamples[94],
      observations: crawlResult.observations.length,
      traceableEntityRatio: crawlTraceable / crawlEntities.length,
      repeatableWithFixedClock: true,
      limitation:
        'Offline CDX fixture with four records. Not a live Common Crawl measurement, not a coverage claim.',
    },
    null,
    2,
  ),
);
