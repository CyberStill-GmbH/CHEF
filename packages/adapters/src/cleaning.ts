import type { ServiceInput } from '../../domain/src/model.js';

export interface DuplicateGroup {
  key: string;
  count: number;
}

export interface CleaningSummary {
  totalInputs: number;
  uniqueServices: number;
  duplicatedInputs: number;
  duplicateGroups: DuplicateGroup[];
}

/**
 * Off-wire duplicate report. Grouping reuses the same
 * address|protocol|port identity as correlation; nothing is hidden or
 * merged. Keys stay in memory: the CLI only prints counts.
 */
export function summarizeDuplicates(
  inputs: readonly ServiceInput[],
): CleaningSummary {
  const counts = new Map<string, number>();
  for (const input of inputs) {
    const key = `${input.address}|${input.protocol}|${input.port}`;
    counts.set(key, (counts.get(key) ?? 0) + 1);
  }
  const duplicateGroups = [...counts.entries()]
    .filter((entry) => (entry[1] ?? 0) > 1)
    .map(([key, count]) => ({ key, count }))
    .sort((a, b) => a.key.localeCompare(b.key));
  const duplicatedInputs = duplicateGroups.reduce(
    (total, group) => total + group.count - 1,
    0,
  );
  return {
    totalInputs: inputs.length,
    uniqueServices: counts.size,
    duplicatedInputs,
    duplicateGroups,
  };
}
