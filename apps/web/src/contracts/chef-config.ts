export interface ChefConfigDto {
  schemaVersion: '1.0.0';
  config: {
    schema_version: 1;
    runtime: {
      active_discovery: false;
      max_workers: number;
      max_events: number;
      max_depth: number;
    };
    scope: {
      allowed_hosts: string[];
      excluded_hosts: string[];
      allowed_ports: number[];
      allowed_protocols: Array<'tcp' | 'udp'>;
      max_scope_distance: number;
    };
    modules: Record<string, boolean>;
    sources: Record<string, { enabled: boolean }>;
    defectdojo: {
      enabled: boolean;
      base_url: string | null;
      engagement_id: number | null;
    };
  };
}
