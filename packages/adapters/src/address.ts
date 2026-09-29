import { isIP } from 'node:net';
import { ChefError } from '../../domain/src/errors.js';
export function canonicalAddress(value: string): string {
  const version = isIP(value);
  if (!version || value.includes('%'))
    throw new ChefError(
      'INVALID_INPUT',
      'Expected a literal IP address without zone ID',
    );
  if (version === 4) return value;
  return new URL(`http://[${value}]/`).hostname.slice(1, -1);
}
