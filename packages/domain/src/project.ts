import { ChefError } from './errors.js';

export type Role = 'owner' | 'member';

export interface Project {
  id: string;
  name: string;
  ownerId: string;
  createdAt: string;
}

export interface Membership {
  projectId: string;
  userId: string;
  role: Role;
}

const MIN_NAME = 2;
const MAX_NAME = 80;

function hasControlCharacters(value: string): boolean {
  for (const character of value) {
    const code = character.codePointAt(0) ?? 0;
    if (code < 0x20 || code === 0x7f) return true;
  }
  return false;
}

/**
 * Normalizes a project name without hiding user input from the audit trail.
 * Rejects control characters so a name cannot forge log lines or break the UI.
 */
export function normalizeProjectName(raw: string): string {
  if (hasControlCharacters(raw))
    throw new ChefError(
      'INVALID_INPUT',
      'Project name contains control characters',
    );
  const name = raw.trim().replace(/\s+/g, ' ');
  if (name.length < MIN_NAME || name.length > MAX_NAME)
    throw new ChefError('INVALID_INPUT', 'Project name length is out of range');
  return name;
}

export function canImport(role: Role): boolean {
  return role === 'owner' || role === 'member';
}

export function canManageMembers(role: Role): boolean {
  return role === 'owner';
}

/** Deny by default: an absent membership is never treated as access. */
export function requireMembership(
  membership: Membership | undefined,
  projectId: string,
): Membership {
  if (membership === undefined)
    throw new ChefError('NOT_FOUND', 'Project not found');
  if (membership.projectId !== projectId)
    throw new ChefError('NOT_FOUND', 'Project not found');
  return membership;
}
