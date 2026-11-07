import { isIP } from 'node:net';
import { ChefError } from '../../domain/src/errors.js';

function expandIPv6(address: string): string {
  if (address.includes('::')) {
    const parts = address.split('::');
    const left = parts[0] ? parts[0].split(':') : [];
    const right = parts[1] ? parts[1].split(':') : [];
    const missing = 8 - left.length - right.length;
    const middle = Array(missing).fill('0000');
    const all = [...left, ...middle, ...right].map((p) => p.padStart(4, '0'));
    return all.join(':');
  }
  return address
    .split(':')
    .map((p) => p.padStart(4, '0'))
    .join(':');
}

export function canonicalAddress(value: string): string {
  const version = isIP(value);
  if (!version || value.includes('%'))
    throw new ChefError(
      'INVALID_INPUT',
      'Expected a literal IP address without zone ID',
    );
  if (version === 4) return value;
  return expandIPv6(value);
}
