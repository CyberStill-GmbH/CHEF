export interface TargetSeedDto {
  schemaVersion: '1.0.0' | '1.1.0';
  kind: 'asn' | 'ipAddress' | 'ipRange' | 'dnsName' | 'url' | 'tcpEndpoint';
  data: string;
  host: string | null;
  port: number | null;
}
