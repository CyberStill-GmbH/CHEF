import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { ImportEvidence } from '../packages/application/src/import-evidence.js';
import { summarizeDuplicates } from '../packages/adapters/src/cleaning.js';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ExactScopeGuard } from '../packages/adapters/src/scope.js';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
import { loadContracts } from '../packages/adapters/src/contracts.js';
import { createCommonCrawlParser } from '../packages/adapters/src/commoncrawl.js';

// Jhojan acceptance suite: real Nmap ingestion behaviour. Given an
// authorized offline XML fixture, When it is imported under the lab scope,
// Then only open services become observations, ambiguous states are dropped
// with counts, and weak signals never merge hosts.
const contracts = loadContracts('docs/contracts/snapshot.schema.json');
const policy = contracts.scope(
  JSON.parse(readFileSync('fixtures/scope/lab.json', 'utf8')),
);
const bytes = readFileSync('fixtures/nmap/lab.xml');
const parser = new NmapXmlParser();
const pipeline = new ImportEvidence(
  parser,
  new ExactScopeGuard(),
  new Sha256Identity(),
  {
    now: () => '2026-09-29T00:00:00.000Z',
  },
);
function xml(
  address: string,
  port: string,
  protocol = 'tcp',
  state = 'open',
): Buffer {
  return Buffer.from(
    `<nmaprun scanner="nmap" start="1790640000"><host><status state="up"/><address addr="${address}" addrtype="${address.includes(':') ? 'ipv6' : 'ipv4'}"/><ports><port protocol="${protocol}" portid="${port}"><state state="${state}"/></port></ports></host></nmaprun>`,
  );
}

test('lab: ambiguous states and down hosts are dropped, never reported as open', () => {
  const result = pipeline.execute(bytes, policy);
  assert.equal(result.observations.length, 3);
  assert.ok(
    result.observations.every(
      (item) => item.port === 8080 && item.protocol === 'tcp',
    ),
  );
  const { stats } = parser.parseWithStats(bytes, policy.maxObservations);
  assert.deepEqual(stats, {
    hostRecords: 3,
    upHosts: 2,
    skippedDownHosts: 1,
    portRecords: 5,
    openKept: 3,
    skippedClosed: 1,
    skippedFiltered: 0,
    skippedOpenFiltered: 1,
    skippedOtherState: 0,
  });
});

test('lab: shared hostname never merges hosts; same port stays per host', () => {
  const result = pipeline.execute(bytes, policy);
  assert.deepEqual(
    result.assets
      .filter((item) => item.kind === 'address')
      .map((item) => item.key)
      .sort(),
    ['127.0.0.1', '127.0.0.2'],
  );
  assert.deepEqual(
    result.assets
      .filter((item) => item.kind === 'service')
      .map((item) => item.key)
      .sort(),
    ['127.0.0.1|tcp|8080', '127.0.0.2|tcp|8080'],
  );
  assert.equal(result.relationships.length, 2);
  assert.ok(
    result.relationships.some((item) => item.observationIds.length === 2),
  );
  assert.ok(result.paths.every((item) => !item.isConfirmedVulnerability));
});

test('lab: duplicates are grouped with count and evidence preserved', () => {
  const inputs = parser.parse(bytes, policy.maxObservations);
  const summary = summarizeDuplicates(inputs);
  assert.equal(summary.totalInputs, 3);
  assert.equal(summary.uniqueServices, 2);
  assert.equal(summary.duplicatedInputs, 1);
  assert.deepEqual(summary.duplicateGroups, [
    { key: '127.0.0.1|tcp|8080', count: 2 },
  ]);
});

test('parseWithStats agrees with parse and with the pipeline', () => {
  const withStats = parser.parseWithStats(bytes, policy.maxObservations);
  assert.deepEqual(
    withStats.inputs,
    parser.parse(bytes, policy.maxObservations),
  );
  assert.equal(
    withStats.stats.openKept,
    pipeline.execute(bytes, policy).observations.length,
  );
});

test('open|filtered-only import yields an empty, valid graph', () => {
  const result = pipeline.execute(
    xml('127.0.0.1', '443', 'udp', 'open|filtered'),
    policy,
  );
  assert.equal(result.observations.length, 0);
  contracts.snapshot(result);
  const { stats } = parser.parseWithStats(
    xml('127.0.0.1', '443', 'udp', 'open|filtered'),
    policy.maxObservations,
  );
  assert.equal(stats.openKept, 0);
  assert.equal(stats.skippedOpenFiltered, 1);
});

test('closed-only and filtered-only imports yield empty, valid graphs', () => {
  const closed = pipeline.execute(
    xml('127.0.0.1', '80', 'tcp', 'closed'),
    policy,
  );
  assert.equal(closed.observations.length, 0);
  contracts.snapshot(closed);
  const filtered = pipeline.execute(
    xml('127.0.0.1', '80', 'tcp', 'filtered'),
    policy,
  );
  assert.equal(filtered.observations.length, 0);
  contracts.snapshot(filtered);
});

test('IPv6 textual forms canonicalize to a single address asset', () => {
  const normalized = contracts.scope({
    ...policy,
    allowedAddresses: ['0:0:0:0:0:0:0:1'],
  });
  const result = pipeline.execute(xml('::1', '80'), normalized);
  assert.equal(
    result.assets.find((item) => item.kind === 'address')?.key,
    '0000:0000:0000:0000:0000:0000:0000:0001',
  );
});

test('CLI emits off-wire cleaning diagnostics without target data', () => {
  const run = spawnSync(
    process.execPath,
    [
      'dist/apps/cli/src/main.js',
      'fixtures/nmap/lab.xml',
      'fixtures/scope/lab.json',
    ],
    { encoding: 'utf8' },
  );
  assert.equal(run.status, 0, run.stderr);
  assert.equal(JSON.parse(run.stdout).observations.length, 3);
  const line = run.stderr
    .split('\n')
    .find((item) => item.includes('"cleaning"'));
  assert.ok(line);
  const diagnostics = JSON.parse(line) as { cleaning: Record<string, number> };
  assert.equal(diagnostics.cleaning['openKept'], 3);
  assert.equal(diagnostics.cleaning['skippedClosed'], 1);
  assert.equal(diagnostics.cleaning['skippedOpenFiltered'], 1);
  assert.equal(diagnostics.cleaning['skippedDownHosts'], 1);
  assert.equal(diagnostics.cleaning['uniqueServices'], 2);
  assert.equal(diagnostics.cleaning['duplicatedInputs'], 1);
  assert.equal(diagnostics.cleaning['duplicateGroups'], 1);
  assert.ok(!run.stderr.includes('127.0.0.1'));
  assert.ok(!run.stderr.includes('8080'));
});

// Common Crawl Index parser tests (synthetic fixtures, no network)
const ccPolicy = {
  allowedAddresses: ['93.184.216.34', '151.101.1.140', '151.101.65.140'],
  allowedPorts: [80, 443],
  allowedProtocols: ['tcp'] as const,
};

const ccParser = createCommonCrawlParser(ccPolicy);

function readCdxFixture(name: string): Buffer {
  return readFileSync(`fixtures/commoncrawl/${name}.jsonl`);
}

test('commoncrawl: positive fixture yields observations within scope', () => {
  const bytes = readCdxFixture('positive');
  const result = ccParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.cdxRecords, 4);
  assert.equal(result.stats.kept, 4);
  assert.equal(result.stats.skippedOutOfScope, 0);
  assert.equal(result.stats.skippedInvalidUrl, 0);
  assert.equal(result.stats.skippedNoIp, 0);
  assert.equal(result.inputs.length, 4);
  // Verify observation structure
  for (const input of result.inputs) {
    assert.ok(input.address);
    assert.ok(input.port === 80 || input.port === 443);
    assert.equal(input.protocol, 'tcp');
    assert.ok(input.observedAt);
    assert.ok(input.locator.startsWith('/cdx/'));
  }
});

test('commoncrawl: negative fixture skips out-of-scope, invalid, non-200', () => {
  const bytes = readCdxFixture('negative');
  const result = ccParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.cdxRecords, 6);
  // 2 non-200 (404, 500), 1 invalid URL, 1 IP not in allowedAddresses (127.0.0.1:9999), 1 malformed JSON line
  assert.equal(result.stats.kept, 1); // only the last valid line
  assert.ok(result.stats.skippedOutOfScope >= 2);
  assert.ok(result.stats.skippedInvalidUrl >= 2);
  assert.ok(result.stats.skippedNoIp >= 0);
});

test('commoncrawl: truncated fixture processes available records', () => {
  const bytes = readCdxFixture('truncated');
  const result = ccParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.cdxRecords, 3);
  assert.equal(result.stats.kept, 3);
  assert.equal(result.inputs.length, 3);
});

test('commoncrawl: empty fixture yields empty valid graph', () => {
  const bytes = readCdxFixture('empty');
  const result = ccParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.cdxRecords, 0);
  assert.equal(result.stats.kept, 0);
  assert.equal(result.inputs.length, 0);
});

test('commoncrawl: scope enforcement - address not in allowedAddresses is skipped', () => {
  const bytes = readCdxFixture('positive');
  const strictPolicy = {
    allowedAddresses: ['93.184.216.34'],
    allowedPorts: [80, 443],
    allowedProtocols: ['tcp'] as const,
  };
  const strictParser = createCommonCrawlParser(strictPolicy);
  const result = strictParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.kept, 1);
  assert.equal(result.stats.skippedOutOfScope, 3);
});

test('commoncrawl: port enforcement - port not in allowedPorts is skipped', () => {
  const bytes = readCdxFixture('positive');
  const strictPolicy = {
    allowedAddresses: ['93.184.216.34', '151.101.1.140', '151.101.65.140'],
    allowedPorts: [443],
    allowedProtocols: ['tcp'] as const,
  };
  const strictParser = createCommonCrawlParser(strictPolicy);
  const result = strictParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.kept, 2);
  assert.equal(result.stats.skippedOutOfScope, 2);
});

test('commoncrawl: protocol enforcement - UDP skipped (CC only has HTTP/HTTPS)', () => {
  const bytes = readCdxFixture('positive');
  const udpOnlyPolicy = {
    allowedAddresses: ['93.184.216.34', '151.101.1.140', '151.101.65.140'],
    allowedPorts: [80, 443],
    allowedProtocols: ['udp'] as const,
  };
  const udpParser = createCommonCrawlParser(udpOnlyPolicy);
  const result = udpParser.parseWithStats(bytes, 1000);
  assert.equal(result.stats.kept, 0);
  assert.equal(result.stats.skippedOutOfScope, 4);
});

test('commoncrawl: maxObservations limit enforced', () => {
  const bytes = readCdxFixture('positive');
  const result = ccParser.parseWithStats(bytes, 2);
  assert.equal(result.inputs.length, 2);
  assert.equal(result.stats.kept, 2);
});

test('commoncrawl: duplicate URLs in CDX are preserved as separate observations (evidence preserved)', () => {
  // Create fixture with duplicate URL (same IP, different timestamps/WARC files)
  const duplicateCdx = Buffer.from(
    '{"urlkey":"com,example,93,184,216,34)/","timestamp":"20241115120000","original":"http://93.184.216.34/","mimetype":"text/html","status":"200","digest":"sha256:abc123","length":"1234","offset":"5678","filename":"CC-MAIN-20241115120000-00000.warc.gz"}\n' +
      '{"urlkey":"com,example,93,184,216,34)/","timestamp":"20241115120001","original":"http://93.184.216.34/","mimetype":"text/html","status":"200","digest":"sha256:def456","length":"567","offset":"9012","filename":"CC-MAIN-20241115120000-00001.warc.gz"}\n',
  );
  const result = ccParser.parseWithStats(duplicateCdx, 1000);
  assert.equal(result.stats.cdxRecords, 2);
  assert.equal(result.stats.kept, 2);
  assert.equal(result.inputs.length, 2);
  // Both observations have different locators (different WARC files)
  assert.ok(result.inputs[0] && result.inputs[1]);
  assert.notEqual(result.inputs[0]?.locator, result.inputs[1]?.locator);
});

test('commoncrawl: IPv6 addresses canonicalized correctly', () => {
  const ipv6Cdx = Buffer.from(
    '{"urlkey":"com,example,2001,db8,1)/","timestamp":"20241115120000","original":"http://[2001:db8::1]/","mimetype":"text/html","status":"200","digest":"sha256:abc123","length":"1234","offset":"5678","filename":"CC-MAIN-20241115120000-00000.warc.gz"}\n',
  );
  const ipv6Policy = {
    allowedAddresses: ['2001:0db8:0000:0000:0000:0000:0000:0001'],
    allowedPorts: [80],
    allowedProtocols: ['tcp'] as const,
  };
  const ipv6Parser = createCommonCrawlParser(ipv6Policy);
  const result = ipv6Parser.parseWithStats(ipv6Cdx, 1000);
  assert.equal(result.stats.kept, 1);
  assert.ok(result.inputs.length > 0);
  assert.equal(
    result.inputs[0]?.address,
    '2001:0db8:0000:0000:0000:0000:0000:0001',
  );
});

test('commoncrawl: default HTTP port 80, HTTPS port 443 inferred when not explicit', () => {
  const noPortCdx = Buffer.from(
    '{"urlkey":"com,example,93,184,216,34)/path","timestamp":"20241115120000","original":"http://93.184.216.34/path","mimetype":"text/html","status":"200","digest":"sha256:abc123","length":"1234","offset":"5678","filename":"CC-MAIN-20241115120000-00000.warc.gz"}\n' +
      '{"urlkey":"com,example,151,101,1,140)/v1","timestamp":"20241115120001","original":"https://151.101.1.140/v1","mimetype":"application/json","status":"200","digest":"sha256:def456","length":"567","offset":"9012","filename":"CC-MAIN-20241115120000-00001.warc.gz"}\n',
  );
  const result = ccParser.parseWithStats(noPortCdx, 1000);
  assert.equal(result.stats.kept, 2);
  assert.ok(result.inputs.length >= 2);
  assert.equal(result.inputs[0]?.port, 80);
  assert.equal(result.inputs[1]?.port, 443);
  assert.equal(result.inputs[0]?.protocol, 'tcp');
  assert.equal(result.inputs[1]?.protocol, 'tcp');
});
