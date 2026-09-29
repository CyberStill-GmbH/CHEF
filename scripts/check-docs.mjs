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
