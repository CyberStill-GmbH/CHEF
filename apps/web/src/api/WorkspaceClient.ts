import type { SnapshotDto } from '../contracts/snapshot';

export interface ProjectSummary {
  project: { id: string; name: string; ownerId: string; createdAt: string };
  role: 'owner' | 'member';
}

export interface WorkspaceClient {
  session(signal?: AbortSignal): Promise<{ userId: string } | null>;
  startSignIn(): Promise<string>;
  logout(): Promise<void>;
  listProjects(signal?: AbortSignal): Promise<ProjectSummary[]>;
  createProject(name: string, signal?: AbortSignal): Promise<ProjectSummary>;
  loadProjectSnapshot(
    projectId: string,
    signal?: AbortSignal,
  ): Promise<SnapshotDto | null>;
  importXml(projectId: string, file: File, signal?: AbortSignal): Promise<void>;
}
