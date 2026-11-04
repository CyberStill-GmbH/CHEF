import { useEffect, useRef, useState } from 'react';

import { prefersReducedMotion } from '../motion';

interface CountUpProps {
  value: number;
  /** Duración en milisegundos de la cuenta. */
  duration?: number;
  /** Retraso inicial, para escalonar varias cifras. */
  delay?: number;
}

/**
 * Número que sube suavemente hasta su valor. Sin movimiento (o sin matchMedia)
 * muestra el valor final desde el primer render.
 */
export function CountUp({ value, duration = 900, delay = 0 }: CountUpProps) {
  const reduced = prefersReducedMotion();
  const [shown, setShown] = useState(reduced ? value : 0);
  const fromRef = useRef(reduced ? value : 0);

  useEffect(() => {
    if (reduced) {
      setShown(value);
      return;
    }
    const from = fromRef.current;
    let frame = 0;
    let start = 0;
    const timer = window.setTimeout(() => {
      const tick = (now: number) => {
        if (start === 0) start = now;
        const progress = Math.min((now - start) / duration, 1);
        const eased = 1 - Math.pow(1 - progress, 3);
        const next = Math.round(from + (value - from) * eased);
        setShown(next);
        fromRef.current = next;
        if (progress < 1) frame = window.requestAnimationFrame(tick);
      };
      frame = window.requestAnimationFrame(tick);
    }, delay);
    return () => {
      window.clearTimeout(timer);
      window.cancelAnimationFrame(frame);
    };
  }, [value, duration, delay, reduced]);

  return <>{shown}</>;
}
