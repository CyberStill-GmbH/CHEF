import type { BbotEventStatisticsDto } from '../contracts/event-statistics';

interface Props {
  statistics: BbotEventStatisticsDto;
}

export function EventStatisticsPanel({ statistics }: Props) {
  return (
    <section
      className="event-statistics-panel"
      aria-labelledby="event-statistics-title"
    >
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
      <div className="statistics-breakdown">
        <details>
          <summary>Eventos por tipo · {statistics.byType.length}</summary>
          {statistics.byType.length === 0 ? (
            <p>Sin eventos.</p>
          ) : (
            <ul>
              {statistics.byType.map(({ eventType, count }) => (
                <li key={eventType}>
                  <code>{eventType}</code>
                  <span>{count}</span>
                </li>
              ))}
            </ul>
          )}
        </details>
        <details>
          <summary>
            Eventos por productor · {statistics.byModule.length}
          </summary>
          {statistics.byModule.length === 0 ? (
            <p>Sin atribuciones de módulo.</p>
          ) : (
            <ul>
              {statistics.byModule.map(({ module, producedTotal, byType }) => (
                <li key={module}>
                  <code>{module}</code>
                  <span>
                    {producedTotal} ·{' '}
                    {byType
                      .map(({ eventType, count }) => `${eventType} ${count}`)
                      .join(', ')}
                  </span>
                </li>
              ))}
            </ul>
          )}
        </details>
      </div>
    </section>
  );
}
