import { readdirSync, readFileSync, existsSync } from 'node:fs';
import { resolve, dirname, relative } from 'node:path';
import assert from 'node:assert/strict';
import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import { parse as parseYaml } from 'yaml';
import { JSDOM } from 'jsdom';

// Mermaid sanitizes labels during parse; provision a DOM before loading its modules.
const dom = new JSDOM('<!doctype html><html><body></body></html>');
globalThis.window = dom.window;
globalThis.document = dom.window.document;
const { default: mermaid } = await import('mermaid');

const root = process.cwd();
const ignored = new Set([
  '.git',
  '.tools',
  'node_modules',
  'dist',
  'artifacts',
  'target',
]);
function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap((entry) => {
    if (ignored.has(entry.name)) return [];
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? files(path) : [path];
  });
}
let markdownCount = 0;
let diagrams = 0;
let skillCount = 0;
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });
for (const file of files(root)) {
  const content = readFileSync(file, 'utf8');
  if (file.endsWith('.json')) JSON.parse(content);
  if (/\.ya?ml$/.test(file)) parseYaml(content);
  if (!file.endsWith('.md')) continue;
  markdownCount++;
  for (const match of content.matchAll(/\[[^\]]*\]\(([^)]+)\)/g)) {
    const link = match[1].replace(/^<|>$/g, '');
    if (/^(https?:|mailto:|#)/.test(link)) continue;
    const path = decodeURIComponent(link.split('#')[0]);
    assert.ok(
      existsSync(resolve(dirname(file), path)),
      `${relative(root, file)}: broken local link ${link}`,
    );
  }
  for (const match of content.matchAll(/```mermaid\s*\n([\s\S]*?)```/g)) {
    await mermaid.parse(match[1]);
    diagrams++;
  }
  if (file.endsWith('SKILL.md')) {
    const match = content.match(/^---\r?\n([\s\S]*?)\r?\n---/);
    assert.ok(match, `Missing frontmatter: ${file}`);
    const metadata = parseYaml(match[1]);
    assert.match(metadata.name, /^[a-z0-9-]{1,64}$/);
    assert.ok(
      typeof metadata.description === 'string' &&
        metadata.description.length > 20,
    );
    assert.equal(metadata.name, dirname(file).split(/[\\/]/).at(-1));
    skillCount++;
  }
}
const schema = JSON.parse(
  readFileSync('docs/contracts/snapshot.schema.json', 'utf8'),
);
const ajv = new Ajv2020({ strict: true, allErrors: true });
addFormats(ajv);
const validate = ajv.compile(schema);
assert.ok(
  validate(
    JSON.parse(readFileSync('docs/contracts/examples/snapshot.json', 'utf8')),
  ),
  JSON.stringify(validate.errors),
);
const reconGraphSchema = JSON.parse(
  readFileSync('docs/contracts/recon-graph.schema.json', 'utf8'),
);
const validateReconGraph = ajv.compile(reconGraphSchema);
const reconGraphExample = JSON.parse(
  readFileSync('docs/contracts/examples/recon-graph.json', 'utf8'),
);
assert.ok(
  validateReconGraph(reconGraphExample),
  JSON.stringify(validateReconGraph.errors),
);
assert.ok(
  reconGraphExample.relationships.some(
    ({ kind, assertion }) =>
      kind === 'parent_domain_of' && assertion === 'hint',
  ),
  'The recon graph example must show parent-domain hints as unverified suggestions.',
);
assert.ok(
  reconGraphExample.assets.some(
    ({ kind, qualification }) => kind === 'domain' && qualification === 'hint',
  ) &&
    reconGraphExample.assets.some(
      ({ kind, qualification }) =>
        kind === 'domain' && qualification === 'unresolved',
    ),
  'Domain hints and unresolved names must retain their distinct qualifications.',
);
assert.ok(
  reconGraphExample.assets.some(
    ({ kind, qualification }) => kind === 'service' && qualification === 'hint',
  ) &&
    reconGraphExample.relationships.some(
      ({ kind, assertion }) =>
        kind === 'exposes_service' && assertion === 'hint',
    ),
  'Common TCP ports must remain service and relationship hints in the recon graph example.',
);
assert.ok(
  reconGraphExample.schemaVersion === '1.4.0' &&
    reconGraphExample.assets.some(
      ({ kind, key, qualification }) =>
        kind === 'organization' && qualification === 'hint' && key === 'bücher',
    ) &&
    reconGraphExample.assets.some(
      ({ kind, key, qualification }) =>
        kind === 'organization' && qualification === 'hint' && key === 'bucher',
    ) &&
    reconGraphExample.evidence.some(
      ({ eventType, module, parentEvidenceId }) =>
        eventType === 'ORG_STUB' &&
        module === 'chef_org_stub_hints' &&
        parentEvidenceId !== null,
    ),
  'Organization names must remain candidate hints with source lineage in recon graph 1.4.0.',
);
const validateScopePolicy = ajv.compile({
  $schema: schema.$schema,
  $defs: schema.$defs,
  $ref: '#/$defs/ScopePolicy',
});
assert.ok(
  validateScopePolicy(
    JSON.parse(readFileSync('docs/contracts/examples/scope-cidr.json', 'utf8')),
  ),
  JSON.stringify(validateScopePolicy.errors),
);
const httpSchema = JSON.parse(
  readFileSync('docs/contracts/http-observation.schema.json', 'utf8'),
);
const validateHttp = ajv.compile(httpSchema);
assert.ok(
  validateHttp(
    JSON.parse(
      readFileSync('docs/contracts/examples/http-observation.json', 'utf8'),
    ),
  ),
  JSON.stringify(validateHttp.errors),
);
const scanLogSchema = JSON.parse(
  readFileSync('docs/contracts/scan-log.schema.json', 'utf8'),
);
const validateScanLog = ajv.compile(scanLogSchema);
assert.ok(
  validateScanLog(
    JSON.parse(readFileSync('docs/contracts/examples/scan-log.json', 'utf8')),
  ),
  JSON.stringify(validateScanLog.errors),
);
const observedDomainExportSchema = JSON.parse(
  readFileSync('docs/contracts/observed-domain-export.schema.json', 'utf8'),
);
const validateObservedDomainExport = ajv.compile(observedDomainExportSchema);
assert.ok(
  validateObservedDomainExport(
    JSON.parse(
      readFileSync(
        'docs/contracts/examples/observed-domain-export.json',
        'utf8',
      ),
    ),
  ),
  JSON.stringify(validateObservedDomainExport.errors),
);
const evidenceCsvExportSchema = JSON.parse(
  readFileSync('docs/contracts/evidence-csv-export.schema.json', 'utf8'),
);
const validateEvidenceCsvExport = ajv.compile(evidenceCsvExportSchema);
const evidenceCsvExample = JSON.parse(
  readFileSync('docs/contracts/examples/evidence-csv-export.json', 'utf8'),
);
assert.ok(
  validateEvidenceCsvExport(evidenceCsvExample),
  JSON.stringify(validateEvidenceCsvExport.errors),
);
const evidenceCsvLines = evidenceCsvExample.csv
  .split('\r\n')
  .filter((line) => line.length > 0);
assert.equal(evidenceCsvLines.length, evidenceCsvExample.rowCount + 1);
assert.ok(evidenceCsvExample.csv.includes('"\'=example.invalid"'));
const nmapEvidenceExportSchema = JSON.parse(
  readFileSync('docs/contracts/nmap-evidence-export.schema.json', 'utf8'),
);
const validateNmapEvidenceExport = ajv.compile(nmapEvidenceExportSchema);
assert.ok(
  validateNmapEvidenceExport(
    JSON.parse(
      readFileSync('docs/contracts/examples/nmap-evidence-export.json', 'utf8'),
    ),
  ),
  JSON.stringify(validateNmapEvidenceExport.errors),
);
const assetInventoryExportSchema = JSON.parse(
  readFileSync('docs/contracts/asset-inventory-export.schema.json', 'utf8'),
);
const validateAssetInventoryExport = ajv.compile(assetInventoryExportSchema);
const assetInventoryExportExample = JSON.parse(
  readFileSync('docs/contracts/examples/asset-inventory-export.json', 'utf8'),
);
assert.ok(
  validateAssetInventoryExport(assetInventoryExportExample),
  JSON.stringify(validateAssetInventoryExport.errors),
);
const assetInventoryCsvLines = assetInventoryExportExample.csv
  .split('\r\n')
  .filter((line) => line.length > 0);
assert.equal(
  assetInventoryCsvLines.length,
  assetInventoryExportExample.rowCount + 1,
);
assert.ok(assetInventoryCsvLines[0].includes('ids_hallazgos_candidatos_json'));
const chefConfigSchema = JSON.parse(
  readFileSync('docs/contracts/chef-config.schema.json', 'utf8'),
);
const validateChefConfig = ajv.compile(chefConfigSchema);
assert.ok(
  validateChefConfig(
    JSON.parse(
      readFileSync('docs/contracts/examples/chef-config.json', 'utf8'),
    ),
  ),
  JSON.stringify(validateChefConfig.errors),
);
const targetSeedSchema = JSON.parse(
  readFileSync('docs/contracts/target-seed.schema.json', 'utf8'),
);
const validateTargetSeed = ajv.compile(targetSeedSchema);
assert.ok(
  validateTargetSeed(
    JSON.parse(
      readFileSync('docs/contracts/examples/target-seed.json', 'utf8'),
    ),
  ),
  JSON.stringify(validateTargetSeed.errors),
);
const asnSeedExample = JSON.parse(
  readFileSync('docs/contracts/examples/target-seed-asn.json', 'utf8'),
);
assert.ok(
  validateTargetSeed(asnSeedExample),
  JSON.stringify(validateTargetSeed.errors),
);
assert.equal(asnSeedExample.kind, 'asn');
assert.equal(asnSeedExample.host, null);
const organizationSeedExample = JSON.parse(
  readFileSync('docs/contracts/examples/target-seed-organization.json', 'utf8'),
);
assert.ok(
  validateTargetSeed(organizationSeedExample),
  JSON.stringify(validateTargetSeed.errors),
);
assert.equal(organizationSeedExample.kind, 'organization');
assert.equal(organizationSeedExample.host, null);
assert.equal(organizationSeedExample.port, null);
const contextualSeedExamples = [
  ['email-address', 'emailAddress', false],
  ['username', 'username', true],
  ['filesystem', 'filesystem', true],
  ['mobile-app', 'mobileApp', true],
  ['blacklist-regex', 'blacklistRegex', true],
];
for (const [name, kind, hasNullHost] of contextualSeedExamples) {
  const example = JSON.parse(
    readFileSync(`docs/contracts/examples/target-seed-${name}.json`, 'utf8'),
  );
  assert.ok(
    validateTargetSeed(example),
    JSON.stringify(validateTargetSeed.errors),
  );
  assert.equal(example.schemaVersion, '1.3.0');
  assert.equal(example.kind, kind);
  if (hasNullHost) {
    assert.equal(example.host, null);
    assert.equal(example.port, null);
  } else {
    assert.equal(typeof example.host, 'string');
  }
}
const eventStatisticsSchema = JSON.parse(
  readFileSync('docs/contracts/event-statistics.schema.json', 'utf8'),
);
const validateEventStatistics = ajv.compile(eventStatisticsSchema);
assert.ok(
  validateEventStatistics(
    JSON.parse(
      readFileSync('docs/contracts/examples/event-statistics.json', 'utf8'),
    ),
  ),
  JSON.stringify(validateEventStatistics.errors),
);
const sources = JSON.parse(readFileSync('docs/research/sources.json', 'utf8'));
assert.equal(new Set(sources.map((item) => item.id)).size, sources.length);
for (const source of sources) {
  assert.equal(new URL(source.url).protocol, 'https:');
  assert.ok(source.consultedAt && source.claim && source.limitation);
}
const requirements = readFileSync('docs/product/requirements.md', 'utf8');
const backlog = readFileSync('docs/plan/backlog.md', 'utf8');
for (const requirement of new Set(backlog.match(/(?:FR|NFR)-\d{2}/g)))
  assert.ok(
    requirements.includes(requirement),
    `Unknown requirement ${requirement}`,
  );
for (const story of new Set(requirements.match(/\b(?:C|W|I|Q|B)\d{2}\b/g)))
  assert.ok(backlog.includes(`## ${story}:`), `Unknown story ${story}`);
console.log(
  `Validated ${markdownCount} Markdown files, ${diagrams} Mermaid diagrams, ${skillCount} skills, JSON/YAML, source registry, examples and requirement/story references.`,
);
