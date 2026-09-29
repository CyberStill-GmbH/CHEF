import { XMLParser, XMLValidator } from 'fast-xml-parser';
import { ChefError } from '../../domain/src/errors.js';
import type { ImportParser } from '../../domain/src/ports.js';
import type { ServiceInput } from '../../domain/src/model.js';
import { canonicalAddress } from './address.js';

function object(value: unknown): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new ChefError('INVALID_INPUT', 'Expected an XML object');
  return value as Record<string, unknown>;
}
function list(value: unknown): unknown[] {
  return value === undefined ? [] : Array.isArray(value) ? value : [value];
}
function text(value: unknown): string {
  if (typeof value !== 'string')
    throw new ChefError('INVALID_INPUT', 'Expected an XML string attribute');
  return value;
}

export class NmapXmlParser implements ImportParser {
  readonly source = 'nmap-xml' as const;
  readonly version = '1.0.0';
  parse(bytes: Uint8Array, maxObservations: number): ServiceInput[] {
    let xml: string;
    try {
      xml = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    } catch {
      throw new ChefError('INVALID_INPUT', 'Input must be valid UTF-8');
    }
    // Native Nmap's harmless declaration is accepted; no external/subset DTD or entities are allowed.
    xml = xml.replace(/<!DOCTYPE\s+nmaprun\s*>/g, '');
    if (/<!DOCTYPE|<!ENTITY/i.test(xml))
      throw new ChefError(
        'INVALID_INPUT',
        'External DTDs and entity declarations are disabled',
      );
    if (XMLValidator.validate(xml) !== true)
      throw new ChefError('INVALID_INPUT', 'Malformed XML');
    let document: unknown;
    try {
      document = new XMLParser({
        ignoreAttributes: false,
        attributeNamePrefix: '',
        parseTagValue: false,
        parseAttributeValue: false,
        processEntities: false,
        allowBooleanAttributes: false,
        maxNestedTags: 32,
      }).parse(xml);
    } catch {
      throw new ChefError(
        'INVALID_INPUT',
        'XML exceeds supported nesting or parser constraints',
      );
    }
    const root = object(object(document).nmaprun);
    if (root.scanner !== 'nmap')
      throw new ChefError('INVALID_INPUT', 'Expected Nmap XML');
    const start = text(root.start);
    if (!/^\d{1,11}$/.test(start))
      throw new ChefError('INVALID_INPUT', 'Nmap start must be Unix seconds');
    const observedAt = new Date(Number(start) * 1000).toISOString();
    const inputs: ServiceInput[] = [];
    const hosts = list(root.host);
    if (hosts.length > maxObservations)
      throw new ChefError('LIMIT_EXCEEDED', 'Too many host records');
    let recordCount = 0;
    for (const [hostIndex, rawHost] of hosts.entries()) {
      const host = object(rawHost);
      if (object(host.status).state !== 'up') continue;
      const addresses = list(host.address)
        .map(object)
        .filter((item) => item.addrtype === 'ipv4' || item.addrtype === 'ipv6');
      if (addresses.length !== 1)
        throw new ChefError(
          'INVALID_INPUT',
          'Each imported host must have exactly one IP address',
        );
      const address = canonicalAddress(text(addresses[0]?.addr));
      const ports =
        host.ports === undefined ? [] : list(object(host.ports).port);
      for (const [portIndex, rawPort] of ports.entries()) {
        if (++recordCount > maxObservations)
          throw new ChefError('LIMIT_EXCEEDED', 'Too many port records');
        const port = object(rawPort);
        const rawNumber = text(port.portid);
        if (!/^\d{1,5}$/.test(rawNumber))
          throw new ChefError('INVALID_INPUT', 'Invalid port number');
        const number = Number(rawNumber);
        if (number < 1 || number > 65535)
          throw new ChefError(
            'INVALID_INPUT',
            'Port must be between 1 and 65535',
          );
        const protocol = port.protocol;
        if (protocol !== 'tcp' && protocol !== 'udp')
          throw new ChefError(
            'INVALID_INPUT',
            'Only TCP and UDP are supported',
          );
        if (object(port.state).state !== 'open') continue;
        inputs.push({
          address,
          port: number,
          protocol,
          observedAt,
          locator: `/nmaprun/host[${hostIndex}]/ports/port[${portIndex}]`,
        });
      }
    }
    return inputs;
  }
}
