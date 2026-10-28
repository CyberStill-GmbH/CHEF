export type ScanLogPhase =
  | 'started'
  | 'event'
  | 'validated'
  | 'stored'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface ScanLogDto {
  schemaVersion: '1.2.0';
  jobId: string;
  requestId: string;
  sequence: number;
  phase: ScanLogPhase;
  message: string;
  eventType?: string;
  observedAt?: string;
}
