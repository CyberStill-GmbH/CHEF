import type {
  ReconAssetDto,
  ReconEvidenceDto,
  ReconRelationshipDto,
} from './recon-graph';

export interface ReconGraphProgressDto {
  schemaVersion: '1.0.0';
  requestId: string;
  jobId: string;
  sequence: number;
  phase: 'evidence' | 'assets' | 'relationships' | 'completed';
  assets: ReconAssetDto[];
  evidence: ReconEvidenceDto[];
  relationships: ReconRelationshipDto[];
  unmappedEventTypes: Record<string, number>;
}
