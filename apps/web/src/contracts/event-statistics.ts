export interface EventTypeCountDto {
  eventType: string;
  count: number;
}

export interface ModuleProducedCountDto {
  module: string;
  producedTotal: number;
  byType: EventTypeCountDto[];
}

export interface BbotEventStatisticsDto {
  totalEvents: number;
  uniqueEventTypes: number;
  modulesRun: number;
  duplicatesSuppressed: number;
  byType: EventTypeCountDto[];
  byModule: ModuleProducedCountDto[];
}

export interface EventStatisticsDto {
  schemaVersion: '1.0.0';
  statistics: BbotEventStatisticsDto;
}
