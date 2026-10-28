import { useEffect, useRef } from 'react';
import { FitAddon } from '@xterm/addon-fit';
import { Terminal } from '@xterm/xterm';

import type { ScanLogDto } from '../contracts/scan-log';
import '@xterm/xterm/css/xterm.css';

interface ScanLogTerminalProps {
  logs: readonly ScanLogDto[];
}

export function ScanLogTerminal({ logs }: ScanLogTerminalProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const writtenCount = useRef(0);

  useEffect(() => {
    const host = hostRef.current;
    if (host === null) return;

    const terminal = new Terminal({
      convertEol: true,
      cursorBlink: false,
      disableStdin: true,
      screenReaderMode: true,
      scrollback: 500,
      theme: { background: '#0f172a', foreground: '#e2e8f0' },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    fit.fit();
    terminalRef.current = terminal;

    const observer = new ResizeObserver(() => fit.fit());
    observer.observe(host);
    return () => {
      observer.disconnect();
      terminal.dispose();
      terminalRef.current = null;
      writtenCount.current = 0;
    };
  }, []);

  useEffect(() => {
    const terminal = terminalRef.current;
    if (terminal === null || logs.length < writtenCount.current) return;
    for (const log of logs.slice(writtenCount.current)) {
      const details = log.eventType === undefined ? '' : ` [${log.eventType}]`;
      terminal.write(`${log.message}${details}\r\n`);
    }
    writtenCount.current = logs.length;
  }, [logs]);

  return (
    <section className="scan-log-panel" aria-labelledby="scan-log-title">
      <h2 id="scan-log-title">Actividad local</h2>
      <div className="scan-log-terminal" ref={hostRef} aria-hidden="true" />
      <ol
        className="scan-log-transcript"
        aria-label="Transcripción de la actividad"
        aria-live="polite"
        aria-relevant="additions"
        role="log"
      >
        {logs.slice(-20).map((log) => (
          <li key={`${log.jobId}-${log.sequence}`}>
            <span>{log.message}</span>
            {log.eventType !== undefined && <code> {log.eventType}</code>}
          </li>
        ))}
      </ol>
    </section>
  );
}
