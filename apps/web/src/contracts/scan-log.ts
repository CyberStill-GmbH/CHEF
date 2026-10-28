export type ScanLogPhase =
  'started' | 'event' | 'validated' | 'stored' | 'completed' | 'failed';

export interface ScanLogDto {
  schemaVersion: '1.0.0';
  jobId: string;
  sequence: number;
  phase: ScanLogPhase;
  message: string;
  eventType?: string;
  observedAt?: string;
}
