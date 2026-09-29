import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { performance } from 'node:perf_hooks';
import { ImportEvidence } from '../packages/application/src/import-evidence.js';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../packages/adapters/src/scope.js';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
import { loadContracts } from '../packages/adapters/src/contracts.js';

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
  { now: () => '2026-09-29T00:00:00.000Z' },
);
const result = pipeline.execute(bytes, policy);
contracts.snapshot(result);
assert.equal(result.observations.length, labels.expectedOpenObservations);
assert.deepEqual(
  result.assets.map((item) => item.key).sort(),
  [...labels.expectedAssetKeys].sort(),
);
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
for (let i = 0; i < 10; i++) pipeline.execute(bytes, policy);
const samples: number[] = [];
for (let i = 0; i < 100; i++) {
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
