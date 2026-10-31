import { useEffect, useRef, useState } from 'react';
import { FitAddon } from '@xterm/addon-fit';
import { Terminal } from '@xterm/xterm';

import type { ScanLogDto } from '../contracts/scan-log';
import '@xterm/xterm/css/xterm.css';
import './ScanLogTerminal.css';

interface ScanLogTerminalProps {
  logs: readonly ScanLogDto[];
  onCommand(command: string): Promise<readonly string[]>;
}

const MAX_COMMAND_CHARS = 8192;
const MAX_HISTORY = 100;
const PROMPT = '\x1b[38;2;128;227;154mchef>\x1b[0m ';

export function ScanLogTerminal({ logs, onCommand }: ScanLogTerminalProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const terminalRef = useRef<Terminal | null>(null);
  const onCommandRef = useRef(onCommand);
  const logsRef = useRef(logs);
  const inputRef = useRef('');
  const historyRef = useRef<string[]>([]);
  const historyCursorRef = useRef(0);
  const lastLogRef = useRef<{ jobId: string; sequence: number } | null>(null);
  const [busy, setBusy] = useState(false);
  const [status, setStatus] = useState('Listo');
  const [platform] = useState(() =>
    /Windows/i.test(navigator.userAgent)
      ? 'Windows'
      : /Linux/i.test(navigator.userAgent)
        ? 'Linux'
        : 'Desktop',
  );

  onCommandRef.current = onCommand;
  logsRef.current = logs;

  useEffect(() => {
    const host = hostRef.current;
    if (host === null) return;

    const terminal = new Terminal({
      convertEol: true,
      cursorBlink: true,
      cursorInactiveStyle: 'outline',
      cursorStyle: 'bar',
      fontFamily: "'Cascadia Code', 'SFMono-Regular', Consolas, monospace",
      fontSize: 12,
      lineHeight: 1.35,
      scrollback: 1000,
      screenReaderMode: true,
      theme: {
        background: '#0b100e',
        foreground: '#d6e4d9',
        cursor: '#80e39a',
        cursorAccent: '#0b100e',
        selectionBackground: '#315a3d',
        black: '#101713',
        brightBlack: '#66736a',
        green: '#80e39a',
        brightGreen: '#a9f2b8',
        red: '#ff8e87',
        brightRed: '#ffaaa4',
      },
    });
    const fit = new FitAddon();
    terminal.loadAddon(fit);
    terminal.open(host);
    terminalRef.current = terminal;

    const fitTerminal = () => {
      if (host.clientWidth > 0 && host.clientHeight > 0) fit.fit();
    };
    fitTerminal();
    const observer = new ResizeObserver(fitTerminal);
    observer.observe(host);

    const writePrompt = () => {
      terminal.write(`\r\x1b[2K${PROMPT}${safeTerminalText(inputRef.current)}`);
    };

    const writeOutput = (lines: readonly string[]) => {
      for (const line of lines) {
        terminal.write(`${safeTerminalText(line)}\r\n`);
      }
    };

    const clearViewport = () => {
      terminal.clear();
      terminal.write('\x1b[H');
    };

    const setInput = (value: string) => {
      inputRef.current = Array.from(value).slice(0, MAX_COMMAND_CHARS).join('');
      writePrompt();
    };

    const replaceFromHistory = (direction: -1 | 1) => {
      const history = historyRef.current;
      if (history.length === 0) return;
      historyCursorRef.current = Math.max(
        0,
        Math.min(history.length, historyCursorRef.current + direction),
      );
      const selected = history[historyCursorRef.current];
      setInput(selected ?? '');
    };

    const runCommand = async (rawCommand: string) => {
      const command = rawCommand.trim();
      if (command.length === 0) {
        writePrompt();
        return;
      }

      if (command === 'clear') {
        clearViewport();
        lastLogRef.current = logsRef.current.at(-1)
          ? {
              jobId: logsRef.current.at(-1)!.jobId,
              sequence: logsRef.current.at(-1)!.sequence,
            }
          : null;
        inputRef.current = '';
        setStatus('Terminal limpia');
        writePrompt();
        return;
      }

      if (!isChefCommand(command)) {
        writeOutput([
          'Comando no reconocido. Usa "chef help" para ver los comandos disponibles.',
        ]);
        setStatus('Comando no reconocido');
        writePrompt();
        return;
      }

      const history = historyRef.current;
      if (history.at(-1) !== command) {
        history.push(command);
        if (history.length > MAX_HISTORY) history.shift();
      }
      historyCursorRef.current = history.length;
      inputRef.current = '';
      setBusy(true);
      setStatus('Ejecutando');
      terminal.options.disableStdin = true;

      try {
        const result = await onCommandRef.current(command);
        writeOutput(result);
        setStatus('Listo');
      } catch {
        writeOutput(['No se pudo ejecutar el comando CHEF.']);
        setStatus('Error al ejecutar');
      } finally {
        setBusy(false);
        terminal.options.disableStdin = false;
        writePrompt();
      }
    };

    writeOutput([
      'CHEF terminal · escribe un comando para empezar',
      'chef help   chef status   chef url <URL>   chef report   clear',
      'chef url clasifica la URL localmente; no consulta la red.',
      '',
    ]);
    writePrompt();
    terminal.focus();

    const inputSubscription = terminal.onData((data) => {
      if (data === '\r') {
        const command = inputRef.current;
        inputRef.current = '';
        terminal.write('\r\n');
        void runCommand(command);
        return;
      }

      if (data === '\u0003') {
        inputRef.current = '';
        terminal.write('^C\r\n');
        setStatus('Entrada cancelada');
        writePrompt();
        return;
      }

      if (data === '\u000c') {
        clearViewport();
        lastLogRef.current = logsRef.current.at(-1)
          ? {
              jobId: logsRef.current.at(-1)!.jobId,
              sequence: logsRef.current.at(-1)!.sequence,
            }
          : null;
        writePrompt();
        return;
      }

      if (data === '\u001b[A') {
        replaceFromHistory(-1);
        return;
      }

      if (data === '\u001b[B') {
        replaceFromHistory(1);
        return;
      }

      if (data === '\u0015') {
        setInput('');
        return;
      }

      if (data === '\u007f' || data === '\b') {
        if (inputRef.current.length > 0) {
          setInput(Array.from(inputRef.current).slice(0, -1).join(''));
        }
        return;
      }

      // Pasted multiline input is never interpreted as a sequence of commands.
      if (data.includes('\n') || data.includes('\r')) {
        inputRef.current = '';
        terminal.write('\r\n');
        writeOutput(['Pega un solo comando por línea.']);
        writePrompt();
        return;
      }

      const printable = Array.from(data).filter((character) => {
        const codePoint = character.codePointAt(0) ?? 0;
        return codePoint >= 0x20 && !(codePoint >= 0x7f && codePoint <= 0x9f);
      });
      if (printable.length === 0) return;

      const remaining = MAX_COMMAND_CHARS - Array.from(inputRef.current).length;
      if (remaining <= 0) return;
      const addition = printable.slice(0, remaining).join('');
      inputRef.current += addition;
      terminal.write(safeTerminalText(addition));
    });

    return () => {
      observer.disconnect();
      inputSubscription.dispose();
      terminal.dispose();
      terminalRef.current = null;
    };
  }, []);

  useEffect(() => {
    const terminal = terminalRef.current;
    if (terminal === null) return;

    const lastLog = lastLogRef.current;
    let startIndex: number;
    if (lastLog !== null) {
      const previousIndex = logs.findIndex(
        (log) =>
          log.jobId === lastLog.jobId && log.sequence === lastLog.sequence,
      );
      startIndex =
        previousIndex < 0 ? Math.max(0, logs.length - 200) : previousIndex + 1;
    } else {
      startIndex = Math.max(0, logs.length - 200);
    }

    const freshLogs = logs.slice(startIndex);
    if (freshLogs.length === 0) return;

    terminal.write('\r\x1b[2K\r\n');
    for (const log of freshLogs) {
      const details = log.eventType === undefined ? '' : ` [${log.eventType}]`;
      terminal.write(
        `${safeTerminalText(log.message)}${safeTerminalText(details)}\r\n`,
      );
    }
    const last = freshLogs.at(-1);
    if (last !== undefined) {
      lastLogRef.current = { jobId: last.jobId, sequence: last.sequence };
    }
    terminal.write(`${PROMPT}${safeTerminalText(inputRef.current)}`);
  }, [logs]);

  return (
    <section
      className="scan-log-panel"
      aria-label={`Terminal CHEF · ${platform}`}
    >
      <div
        className="scan-log-terminal"
        ref={hostRef}
        role="application"
        aria-label="Terminal interactiva de CHEF. Escribe chef help para ver los comandos."
      />
      <span className="terminal-announcement" role="status" aria-live="polite">
        {busy ? 'Ejecutando comando CHEF' : status}
      </span>
    </section>
  );
}

function isChefCommand(command: string) {
  return (
    command === 'chef help' ||
    command === 'chef status' ||
    command === 'chef report' ||
    /^chef url\s+\S.{0,8190}$/.test(command)
  );
}

function safeTerminalText(value: string) {
  return Array.from(value, (character) =>
    character < ' ' || (character >= '\u007f' && character <= '\u009f')
      ? ' '
      : character,
  )
    .join('')
    .slice(0, 4000);
}
