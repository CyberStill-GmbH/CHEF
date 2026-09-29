import type { ScopeGuard } from '../../domain/src/ports.js';
import type { ScopePolicy, ServiceInput } from '../../domain/src/model.js';
import { ChefError } from '../../domain/src/errors.js';
import { canonicalAddress } from './address.js';
export class ExactScopeGuard implements ScopeGuard {
  check(input: ServiceInput, policy: ScopePolicy): void {
    const address = canonicalAddress(input.address);
    if (
      policy.excludedAddresses.includes(address) ||
      !policy.allowedAddresses.includes(address) ||
      !policy.allowedPorts.includes(input.port) ||
      !policy.allowedProtocols.includes(input.protocol)
    ) {
      throw new ChefError(
        'OUT_OF_SCOPE',
        'An imported open service is outside the explicit scope policy',
      );
    }
  }
}
