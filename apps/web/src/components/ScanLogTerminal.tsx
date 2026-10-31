import { useEffect, useRef, useState, type FormEvent } from 'react';
import { FitAddon } from '@xterm/addon-fit';
import { Terminal } from '@xterm/xterm';

import type { ScanLogDto } from '../contracts/scan-log';
import '@xterm/xterm/css/xterm.css';

interface ScanLogTerminalProps {
  logs: readonly ScanLogDto[];
  onCommand(command: string): Promise<readonly string[]>;
}

interface CommandEntry {
  id: number;
  command: string;
  result: readonly string[];
}

export function ScanLogTerminal({ logs, onCommand }: ScanLogTerminalProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const writtenCount = useRef(0);
  const commandId = useRef(0);
  const [input, setInput] = useState('');
  const [busy, setBusy] = useState(false);
  const [entries, setEntries] = useState<CommandEntry[]>([]);
  const [status, setStatus] = useState('');
  const [clearedLogCount, setClearedLogCount] = useState(0);

  useEffect(() => {
    const host = hostRef.current;
    if (host === null) return;

    const terminal = new Terminal({
      convertEol: true,
      cursorBlink: false,
      disableStdin: true,
      screenReaderMode: true,
      scrollback: 500,
      theme: {
        background: '#0c120e',
        foreground: '#d6f5dc',
        cursor: '#80e39a',
        selectionBackground: '#315a3d',
      },
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
      terminal.write(`${safeTerminalText(log.message)}${safeTerminalText(details)}\r\n`);
    }
    writtenCount.current = logs.length;
  }, [logs]);

  async function submitCommand(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    const command = input.trim();
    if (command.length === 0 || busy) return;
    if (command === 'clear') {
      terminalRef.current?.clear();
      writtenCount.current = logs.length;
      setClearedLogCount(logs.length);
      setEntries([]);
      setInput('');
      setStatus('Terminal limpia.');
      return;
    }

    setInput('');
    setBusy(true);
    setStatus('Ejecutando comando CHEF…');
    try {
      const result = await onCommand(command);
      setEntries((current) => [
        ...current.slice(-49),
        { id: commandId.current++, command: safeDisplayCommand(command), result },
      ]);
      setStatus('');
    } catch {
      setEntries((current) => [
        ...current.slice(-49),
        {
          id: commandId.current++,
          command: safeDisplayCommand(command),
          result: ['No se pudo ejecutar el comando.'],
        },
      ]);
      setStatus('');
    } finally {
      setBusy(false);
    }
  }

  const platform = /Windows/i.test(navigator.userAgent)
    ? 'Windows'
    : /Linux/i.test(navigator.userAgent)
      ? 'Linux'
      : 'Desktop';

  return (
    <section className="scan-log-panel" aria-labelledby="scan-log-title">
      <header className="terminal-heading">
        <h2 id="scan-log-title">Terminal CHEF</h2>
        <span>{platform} · solo comandos CHEF</span>
      </header>
      <div className="scan-log-terminal" ref={hostRef} aria-hidden="true" />
      <ol
        className="chef-command-output"
        aria-label="Resultado de comandos"
        aria-live="polite"
        role="log"
      >
        {entries.map((entry) => (
          <li key={entry.id}>
            <code>chef&gt; {entry.command}</code>
            {entry.result.map((line, index) => (
              <p key={`${entry.id}-${index}`}>{line}</p>
            ))}
          </li>
        ))}
      </ol>
      <ol
        className="scan-log-transcript"
        aria-label="Transcripción de la actividad"
        aria-live="polite"
        aria-relevant="additions"
        role="log"
      >
        {logs.slice(clearedLogCount).slice(-20).map((log) => (
          <li key={`${log.jobId}-${log.sequence}`}>
            <span>{log.message}</span>
            {log.eventType !== undefined && <code> {log.eventType}</code>}
          </li>
        ))}
      </ol>
      <form className="chef-command-form" onSubmit={submitCommand}>
        <label htmlFor="chef-command-input">CHEF/{platform}&gt;</label>
        <input
          id="chef-command-input"
          value={input}
          onChange={(event) => setInput(event.currentTarget.value)}
          disabled={busy}
          autoComplete="off"
          spellCheck={false}
          maxLength={8192}
          placeholder="chef help"
          aria-describedby="chef-command-status"
        />
        <button type="submit" disabled={busy || input.trim().length === 0}>
          {busy ? '…' : 'Ejecutar'}
        </button>
      </form>
      <p id="chef-command-status" className="terminal-status" role="status">
        {status || 'chef help · chef status · chef url <URL> · chef report · clear'}
      </p>
    </section>
  );
}

function safeTerminalText(value: string) {
  return value.replace(/[\u0000-\u001f\u007f-\u009f]/g, ' ').slice(0, 4000);
}

function safeDisplayCommand(command: string) {
  if (!command.startsWith('chef url ')) return command;
  try {
    return `chef url ${new URL(command.slice('chef url '.length)).origin}`;
  } catch {
    return 'chef url [URL]';
  }
}
