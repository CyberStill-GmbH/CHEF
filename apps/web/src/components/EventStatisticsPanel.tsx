import type { BbotEventStatisticsDto } from '../contracts/event-statistics';

interface Props {
  statistics: BbotEventStatisticsDto;
}

export function EventStatisticsPanel({ statistics }: Props) {
  return (
    <section aria-labelledby="event-statistics-title">
      <h2 id="event-statistics-title">Estadísticas del archivo de eventos</h2>
      <p>
        Resumen del snapshot importado. No representa actividad en tiempo real.
      </p>
      <dl>
        <dt>Eventos</dt>
        <dd>{statistics.totalEvents}</dd>
        <dt>Tipos de evento</dt>
        <dd>{statistics.uniqueEventTypes}</dd>
        <dt>Módulos registrados</dt>
        <dd>{statistics.modulesRun}</dd>
        <dt>Duplicados suprimidos</dt>
        <dd>{statistics.duplicatesSuppressed}</dd>
      </dl>
      <h3>Eventos por tipo</h3>
      {statistics.byType.length === 0 ? (
        <p>Sin eventos.</p>
      ) : (
        <ul>
          {statistics.byType.map(({ eventType, count }) => (
            <li key={eventType}>
              {eventType}: {count}
            </li>
          ))}
        </ul>
      )}
      <h3>Eventos atribuidos a productor</h3>
      {statistics.byModule.length === 0 ? (
        <p>Sin atribuciones de módulo.</p>
      ) : (
        <ul>
          {statistics.byModule.map(({ module, producedTotal, byType }) => (
            <li key={module}>
              {module}: {producedTotal} (
              {byType
                .map(({ eventType, count }) => `${eventType} ${count}`)
                .join(', ')}
              )
            </li>
          ))}
        </ul>
      )}
    </section>
  );
}
