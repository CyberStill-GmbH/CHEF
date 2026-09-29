import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormats from 'ajv-formats';
import { Sha256Identity } from '../packages/adapters/src/identity.js';
test('proposed HTTP golden envelope has valid schema and portable identity', () => {
  const ajv = new Ajv2020({ strict: true, allErrors: true });
  addFormats.default(ajv);
  const schema: object = JSON.parse(
    readFileSync('docs/contracts/http-observation.schema.json', 'utf8'),
  );
  const validate = ajv.compile<{
    schemaVersion: string;
    observations: {
      id: string;
      method: string;
      origin: string;
      pathSha256: string;
      evidence: { sha256: string };
    }[];
  }>(schema);
  const envelope: unknown = JSON.parse(
    readFileSync('fixtures/http/golden.json', 'utf8'),
  );
  assert.ok(validate(envelope), JSON.stringify(validate.errors));
  const identity = new Sha256Identity();
  const item = envelope.observations[0];
  assert.ok(item);
  assert.equal(item.pathSha256, identity.digest('/health'));
  assert.equal(
    item.id,
    identity.id('observation', [
      'http-endpoint/v1',
      item.method,
      item.origin,
      item.pathSha256,
    ]),
  );
  assert.equal(
    item.evidence.sha256,
    identity.digest(
      JSON.stringify([item.method, item.origin, item.pathSha256]),
    ),
  );
  assert.ok(!validate({ ...envelope, schemaVersion: '1.0.0' }));
});
