export interface TargetSeedDto {
  schemaVersion: '1.0.0' | '1.1.0' | '1.2.0' | '1.3.0';
  kind:
    | 'asn'
    | 'organization'
    | 'emailAddress'
    | 'username'
    | 'filesystem'
    | 'mobileApp'
    | 'blacklistRegex'
    | 'ipAddress'
    | 'ipRange'
    | 'dnsName'
    | 'url'
    | 'tcpEndpoint';
  data: string;
  host: string | null;
  port: number | null;
}
