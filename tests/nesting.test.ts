import assert from 'node:assert/strict';
import { test } from 'node:test';
import { NmapXmlParser } from '../packages/adapters/src/nmap.js';
import { ChefError } from '../packages/domain/src/errors.js';
test('excessively nested XML is rejected without unbounded tree construction', () => {
  const xml =
    '<nmaprun scanner="nmap" start="1790640000">' +
    '<a>'.repeat(100) +
    '</a>'.repeat(100) +
    '</nmaprun>';
  assert.throws(
    () => new NmapXmlParser().parse(Buffer.from(xml), 100),
    (error: unknown) =>
      error instanceof ChefError && error.code === 'INVALID_INPUT',
  );
});
