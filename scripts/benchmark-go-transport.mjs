import { readFileSync } from 'node:fs';
import { cpus, platform, release, arch } from 'node:os';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { GoCorrelationEngine } from '../dist/packages/adapters/src/go-correlation.js';

const input = JSON.parse(
  readFileSync('docs/contracts/examples/snapshot.json', 'utf8'),
);
const engine = new GoCorrelationEngine({
  executable: resolve(
    `artifacts/bin/correlate${process.platform === 'win32' ? '.exe' : ''}`,
  ),
});
const samples = [];
let nodeRssPeakObserved = process.memoryUsage().rss;
for (let i = 0; i < 20; i++) {
  const start = performance.now();
  await engine.correlate(input);
  samples.push(performance.now() - start);
  nodeRssPeakObserved = Math.max(
    nodeRssPeakObserved,
    process.memoryUsage().rss,
  );
}
samples.sort((a, b) => a - b);
console.log(
  JSON.stringify(
    {
      corpus: 'docs/contracts/examples/snapshot.json',
      observations: input.observations.length,
      samples: samples.length,
      cpu: cpus()[0]?.model,
      os: `${platform()} ${release()} ${arch()}`,
      node: process.version,
      latencyMs: {
        min: samples[0],
        p50: samples[9],
        p95: samples[18],
        max: samples[19],
      },
      nodeRssPeakObservedBytes: nodeRssPeakObserved,
      limitations:
        'Small offline fixture; RSS is sampled in Node, not peak Go memory. No production performance claim.',
    },
    null,
    2,
  ),
);
