import { useEffect, useRef, useState } from 'react';

import type { ApiClient } from '../api/ApiClient';
import type { TargetSeedDto } from '../contracts/target-seed';

interface Props {
  client: ApiClient;
}

const kindLabels: Record<TargetSeedDto['kind'], string> = {
  asn: 'Sistema autónomo (ASN)',
  organization: 'Organización candidata',
  ipAddress: 'Dirección IP',
  ipRange: 'Rango CIDR',
  dnsName: 'Nombre DNS',
  url: 'URL no verificada',
  tcpEndpoint: 'Endpoint TCP objetivo',
};

export function TargetSeedPanel({ client }: Props) {
  const [input, setInput] = useState('');
  const [result, setResult] = useState<TargetSeedDto | null>(null);
  const [status, setStatus] = useState('');
  const [busy, setBusy] = useState(false);
  const requestRef = useRef<AbortController | null>(null);

  useEffect(() => () => requestRef.current?.abort(), []);

  async function parseTarget() {
    if (!client.parseTargetSeed || input.trim().length === 0 || busy) return;

    const controller = new AbortController();
    requestRef.current = controller;
    setBusy(true);
    setResult(null);
    setStatus('Clasificando el objetivo localmente…');
    try {
      const parsed = await client.parseTargetSeed(input, {
        signal: controller.signal,
      });
      if (controller.signal.aborted) return;
      setResult(parsed);
      setStatus('Objetivo clasificado. No se consultó el host.');
    } catch (error) {
      if (controller.signal.aborted) return;
      setStatus(
        error instanceof Error
          ? error.message
          : 'CHEF no pudo clasificar este objetivo.',
      );
    } finally {
      if (!controller.signal.aborted) setBusy(false);
      if (requestRef.current === controller) requestRef.current = null;
    }
  }

  return (
    <section aria-labelledby="target-seed-title" className="target-seed-panel">
      <h2 id="target-seed-title">Preparar objetivo</h2>
      <p>
        Rust clasifica ASN, organización, IP, CIDR, dominio, URL o endpoint
        TCP. La vista previa ASN no expande rangos ni consulta fuentes. Una
        organización es solo una etiqueta aportada por el usuario: no confirma
        propiedad. Ninguna entrada se agrega al scope, confirma existencia o
        inicia reconocimiento.
      </p>
      <form
        onSubmit={(event) => {
          event.preventDefault();
          void parseTarget();
        }}
      >
        <label htmlFor="chef-target-input">Objetivo literal</label>
        <input
          id="chef-target-input"
          type="text"
          autoComplete="off"
          spellCheck={false}
          maxLength={8192}
          placeholder="portal.example.invalid, https://portal.example.invalid o ORG:Acme"
          value={input}
          disabled={busy}
          onChange={(event) => {
            setInput(event.currentTarget.value);
            setResult(null);
            setStatus('');
          }}
        />
        <button type="submit" disabled={busy || input.trim().length === 0}>
          {busy ? 'Clasificando…' : 'Clasificar'}
        </button>
      </form>
      <p role="status" aria-live="polite">
        {status}
      </p>
      {result !== null && (
        <dl className="target-seed-result">
          <dt>Tipo</dt>
          <dd>{kindLabels[result.kind]}</dd>
          <dt>Valor CHEF</dt>
          <dd className="target-seed-value">{result.data}</dd>
          {result.host !== null && (
            <>
              <dt>Host</dt>
              <dd>{result.host}</dd>
            </>
          )}
          {result.port !== null && (
            <>
              <dt>Puerto objetivo</dt>
              <dd>{result.port}</dd>
            </>
          )}
        </dl>
      )}
    </section>
  );
}
