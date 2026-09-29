import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { ImportEvidence } from '../packages/application/src/import-evidence.js';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../packages/adapters/src/scope.js';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
import { loadContracts } from '../packages/adapters/src/contracts.js';
import { ChefError } from '../packages/domain/src/errors.js';
import { correlate } from '../packages/domain/src/correlate.js';
import type { Snapshot } from '../packages/domain/src/model.js';

const contracts = loadContracts('docs/contracts/snapshot.schema.json');
const policy = contracts.scope(
  JSON.parse(readFileSync('fixtures/scope/lab.json', 'utf8')),
);
const bytes = readFileSync('fixtures/nmap/lab.xml');
const identity = new Sha256Identity();
const parser = new NmapXmlParser();
const pipeline = new ImportEvidence(parser, new ExactScopeGuard(), identity, {
  now: () => '2026-09-29T00:00:00.000Z',
});
function code(expected: ChefError['code']) {
  return (error: unknown) =>
    error instanceof ChefError && error.code === expected;
}
function xml(address: string, port: string, protocol = 'tcp') {
  return Buffer.from(
    `<nmaprun scanner="nmap" start="1790640000"><host><status state="up"/><address addr="${address}" addrtype="${address.includes(':') ? 'ipv6' : 'ipv4'}"/><ports><port protocol="${protocol}" portid="${port}"><state state="open"/></port></ports></host></nmaprun>`,
  );
}

test('lab: only open ports are correlated; distinct hosts never merge', () => {
  const result = pipeline.execute(bytes, policy);
  contracts.snapshot(result);
  assert.equal(result.observations.length, 3);
  assert.equal(result.assets.length, 4);
  assert.equal(result.relationships.length, 2);
  assert.equal(result.paths.length, 2);
  assert.ok(result.paths.every((item) => !item.isConfirmedVulnerability));
  assert.ok(
    result.relationships.some((item) => item.observationIds.length === 2),
  );
  const observations = new Map(
    result.observations.map((item) => [item.id, item]),
  );
  for (const asset of result.assets)
    assert.ok(asset.observationIds.every((id) => observations.has(id)));
  for (const relation of result.relationships) {
    assert.ok(result.assets.some((asset) => asset.id === relation.from));
    assert.ok(result.assets.some((asset) => asset.id === relation.to));
    assert.ok(relation.observationIds.every((id) => observations.has(id)));
  }
});

test('repeatable snapshot with fixed clock; correlation commutes and is idempotent', () => {
  const result = pipeline.execute(bytes, policy);
  assert.deepEqual(result, pipeline.execute(bytes, policy));
  const graph = correlate(result.observations, identity);
  assert.deepEqual(
    graph,
    correlate([...result.observations].reverse(), identity),
  );
  assert.deepEqual(
    graph,
    correlate([...result.observations, ...result.observations], identity),
  );
});

test('external DTD/entity imports and malformed XML fail before publishing', () => {
  const hostile = readFileSync('fixtures/nmap/hostile-xxe.xml');
  assert.throws(() => pipeline.execute(hostile, policy), code('INVALID_INPUT'));
  assert.throws(
    () => pipeline.execute(Buffer.from('<nmaprun>'), policy),
    code('INVALID_INPUT'),
  );
  assert.throws(
    () =>
      pipeline.execute(
        Buffer.from(
          '<!DOCTYPE nmaprun SYSTEM "https://example.invalid/dtd"><nmaprun/>',
        ),
        policy,
      ),
    code('INVALID_INPUT'),
  );
  assert.throws(
    () => pipeline.execute(new Uint8Array([0xff]), policy),
    code('INVALID_INPUT'),
  );
});

test('out of scope address, exclusion, port, or protocol fails closed', () => {
  assert.throws(
    () => pipeline.execute(xml('192.0.2.1', '80'), policy),
    code('OUT_OF_SCOPE'),
  );
  assert.throws(
    () =>
      pipeline.execute(bytes, { ...policy, excludedAddresses: ['127.0.0.2'] }),
    code('OUT_OF_SCOPE'),
  );
  assert.throws(
    () => pipeline.execute(xml('127.0.0.1', '4444'), policy),
    code('OUT_OF_SCOPE'),
  );
  assert.throws(
    () =>
      pipeline.execute(xml('127.0.0.1', '80', 'udp'), {
        ...policy,
        allowedProtocols: ['tcp'],
      }),
    code('OUT_OF_SCOPE'),
  );
});

test('input, observation limits and cancellation are enforced', () => {
  assert.throws(
    () => pipeline.execute(bytes, { ...policy, maxInputBytes: 1 }),
    code('LIMIT_EXCEEDED'),
  );
  assert.throws(
    () => pipeline.execute(bytes, { ...policy, maxObservations: 1 }),
    code('LIMIT_EXCEEDED'),
  );
  const controller = new AbortController();
  controller.abort();
  assert.throws(
    () => pipeline.execute(bytes, policy, controller.signal),
    code('CANCELLED'),
  );
});

test('invalid scope and unsafe port/address values are rejected', () => {
  assert.throws(
    () => contracts.scope({ ...policy, allowedAddresses: [] }),
    code('INVALID_INPUT'),
  );
  assert.throws(
    () => contracts.scope({ ...policy, maxInputBytes: 3_000_000 }),
    code('INVALID_INPUT'),
  );
  assert.throws(
    () => contracts.scope({ ...policy, extra: true }),
    code('INVALID_INPUT'),
  );
  for (const port of ['0', '65536', '1e2', '-1', '80x'])
    assert.throws(
      () => parser.parse(xml('127.0.0.1', port), 100),
      code('INVALID_INPUT'),
    );
  assert.throws(
    () => parser.parse(xml('localhost', '80'), 100),
    code('INVALID_INPUT'),
  );
});

test('IPv6 identity canonicalization prevents textual duplicate assets', () => {
  const normalized = contracts.scope({
    ...policy,
    allowedAddresses: ['0:0:0:0:0:0:0:1'],
  });
  const result = pipeline.execute(xml('::1', '80'), normalized);
  assert.equal(
    result.assets.find((item) => item.kind === 'address')?.key,
    '::1',
  );
});

test('tampered snapshots and unsupported versions fail contract validation', () => {
  const result: Snapshot = pipeline.execute(bytes, policy);
  assert.throws(
    () => contracts.snapshot({ ...result, schemaVersion: '2.0.0' }),
    code('INVALID_INPUT'),
  );
  const tampered = structuredClone(result);
  const path = tampered.paths[0];
  assert.ok(path);
  Object.assign(path, { isConfirmedVulnerability: true });
  assert.throws(() => contracts.snapshot(tampered), code('INVALID_INPUT'));
});
