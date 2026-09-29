import { readFileSync } from 'node:fs';
import { Ajv2020 } from 'ajv/dist/2020.js';
import addFormatsModule from 'ajv-formats';
import { ChefError } from '../../domain/src/errors.js';
import type { ScopePolicy, Snapshot } from '../../domain/src/model.js';
import { canonicalAddress } from './address.js';

const ajv = new Ajv2020({ allErrors: true, strict: true });
addFormatsModule.default(ajv);
// URL relative to source AND compiled output; the CLI provides the repository schema path explicitly.
export function loadContracts(schemaFile: string) {
  const schema: object = JSON.parse(readFileSync(schemaFile, 'utf8'));
  ajv.removeSchema('https://chef.local/schema/1.0.0');
  ajv.addSchema(schema);
  const scope = ajv.compile({
    $ref: 'https://chef.local/schema/1.0.0#/$defs/ScopePolicy',
  });
  const snapshot = ajv.getSchema('https://chef.local/schema/1.0.0');
  if (!snapshot) throw new Error('Snapshot schema missing');
  return {
    scope(value: unknown): ScopePolicy {
      if (!scope(value))
        throw new ChefError(
          'INVALID_INPUT',
          'Scope policy fails schema validation',
        );
      const policy = structuredClone(value) as ScopePolicy;
      policy.allowedAddresses = [
        ...new Set(policy.allowedAddresses.map(canonicalAddress)),
      ].sort();
      policy.excludedAddresses = [
        ...new Set(policy.excludedAddresses.map(canonicalAddress)),
      ].sort();
      policy.allowedPorts.sort((a, b) => a - b);
      policy.allowedProtocols.sort();
      return policy;
    },
    snapshot(value: unknown): Snapshot {
      if (!snapshot(value))
        throw new ChefError(
          'INVALID_INPUT',
          'Snapshot fails schema validation',
        );
      return value as Snapshot;
    },
  };
}
