import { ChefError } from '../../domain/src/errors.js';
import type { ScopePolicy, Snapshot } from '../../domain/src/model.js';
import { canImport, requireMembership } from '../../domain/src/project.js';
import type {
  ProjectRepository,
  SnapshotStore,
  CorrelationEngine,
} from '../../domain/src/ports.js';
import type { ImportEvidence } from './import-evidence.js';

export interface ImportIntoProjectInput {
  projectId: string;
  userId: string;
  bytes: Uint8Array;
  policy: ScopePolicy;
  signal?: AbortSignal;
}

export class ImportIntoProject {
  constructor(
    private readonly projects: ProjectRepository,
    private readonly snapshots: SnapshotStore,
    private readonly importer: ImportEvidence,
    private readonly engine?: CorrelationEngine,
  ) {}

  /**
   * Membership is resolved before the parser runs so an outsider cannot use
   * parse errors, timing, or byte limits to probe another team's project.
   */
  async execute(input: ImportIntoProjectInput): Promise<Snapshot> {
    const membership = requireMembership(
      await this.projects.findMembership(input.projectId, input.userId),
      input.projectId,
    );
    if (!canImport(membership.role))
      throw new ChefError('FORBIDDEN', 'Role cannot import evidence');
    const project = await this.projects.findProject(input.projectId);
    if (project === undefined)
      throw new ChefError('NOT_FOUND', 'Project not found');
    let snapshot: Snapshot;
    if (this.engine === undefined) {
      snapshot = this.importer.execute(input.bytes, input.policy, input.signal);
    } else {
      const prepared = this.importer.prepare(
        input.bytes,
        input.policy,
        input.signal,
      );
      snapshot = {
        ...prepared,
        ...(await this.engine.correlate(prepared, input.signal)),
      };
    }
    if (input.signal?.aborted)
      throw new ChefError('CANCELLED', 'Import cancelled');
    await this.snapshots.save(input.projectId, snapshot);
    return snapshot;
  }
}
