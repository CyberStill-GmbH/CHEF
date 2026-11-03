import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import { auroraTone, type AuroraPreset } from './AuroraBackground';

const RING_COUNT = 7;
const CLICK_SECONDS = 1.8;
/** Cada marca grande del anillo cae cada 30°: así "encaja" al hacer clic. */
const DETENT = Math.PI / 6;

interface Ring {
  /** Radio como fracción del radio mayor. */
  radius: number;
  speed: number;
  /** Cuánto se desplaza el anillo con el cursor (profundidad). */
  depth: number;
  /** Cuánto gira al hacer scroll. */
  scrollTurn: number;
  style: 'ticks' | 'arcs' | 'dots' | 'plain';
  nodes: number;
  phase: number;
  /** Giro extra que se ganó al encajar con un clic. */
  snap: number;
  snapTarget: number;
}

interface Pulse {
  x: number;
  y: number;
  start: number;
}

function seeded(seed: number): () => number {
  let state = seed >>> 0;
  return () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/**
 * Fondo de Ajustes: un instrumento de anillos concéntricos, entre astrolabio y
 * mecanismo de relojería. Cada anillo lleva marcas, arcos o nodos y gira a su
 * ritmo; el cursor los desplaza con profundidad, el scroll los hace girar y un
 * clic hace que el anillo más cercano encaje en su siguiente marca mientras una
 * onda se expande desde ese punto. Canvas 2D propio, con el color del ambiente.
 */
export function InstrumentStage({ preset }: { preset: AuroraPreset }) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null || typeof window.matchMedia !== 'function') return;
    const context = canvas.getContext('2d');
    if (context === null) return;

    const reduced = prefersReducedMotion();
    const tone = auroraTone(preset);
    const light = Math.min(tone.lightness + 8, 88);
    const color = (alpha: number, shift = 0, lift = 0) =>
      `hsla(${(tone.hue + shift + 360) % 360}, ${tone.saturation}%, ${Math.min(light + lift, 96)}%, ${alpha})`;

    const random = seeded(59);
    const styles: Ring['style'][] = [
      'plain',
      'ticks',
      'dots',
      'arcs',
      'ticks',
      'plain',
      'arcs',
    ];
    const rings: Ring[] = Array.from({ length: RING_COUNT }, (_, index) => ({
      radius: 0.22 + index * 0.135,
      speed: (index % 2 === 0 ? 1 : -1) * (0.018 + random() * 0.05),
      depth: 6 + index * 5,
      scrollTurn: (index % 2 === 0 ? 1 : -1) * (0.0004 + index * 0.00012),
      style: styles[index] ?? 'plain',
      nodes: 3 + Math.floor(random() * 4),
      phase: random() * Math.PI * 2,
      snap: 0,
      snapTarget: 0,
    }));
    const pulses: Pulse[] = [];
    const pointer = { x: 0.5, y: 0.5, sx: 0.5, sy: 0.5 };

    const scroller = document.querySelector<HTMLElement>('.main-content');
    const readScroll = () => {
      if (
        scroller !== null &&
        scroller.scrollHeight - scroller.clientHeight > 1
      )
        return scroller.scrollTop;
      return window.scrollY;
    };

    let width = 0;
    let height = 0;
    let frame = 0;
    let last = 0;
    let dirty = true;

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      dirty = true;
    };

    const draw = (seconds: number) => {
      context.clearRect(0, 0, width, height);
      const major = Math.max(width, height) * 0.72;
      const baseX = width * 0.72;
      const baseY = height * 0.5;
      const scrollTurn = readScroll();
      context.globalCompositeOperation = 'lighter';

      // Resplandor suave detrás del instrumento.
      const halo = context.createRadialGradient(
        baseX,
        baseY,
        0,
        baseX,
        baseY,
        major * 0.8,
      );
      halo.addColorStop(0, color(0.1, 0, -8));
      halo.addColorStop(1, color(0, 0, -8));
      context.fillStyle = halo;
      context.fillRect(0, 0, width, height);

      // Retícula en cruz que atraviesa el instrumento.
      context.strokeStyle = color(0.07, 0, 4);
      context.lineWidth = 1;
      context.beginPath();
      context.moveTo(baseX, 0);
      context.lineTo(baseX, height);
      context.moveTo(0, baseY);
      context.lineTo(width, baseY);
      context.stroke();

      rings.forEach((ring, index) => {
        const radius = ring.radius * major;
        const parallaxX = (pointer.sx - 0.5) * ring.depth;
        const parallaxY = (pointer.sy - 0.5) * ring.depth;
        const cx = baseX + parallaxX;
        const cy = baseY + parallaxY;
        const spin = reduced
          ? ring.phase
          : ring.phase + seconds * ring.speed + scrollTurn * ring.scrollTurn;
        const turn = spin + ring.snap;

        // Onda de un clic: ilumina el anillo cuando la onda lo cruza.
        let wave = 0;
        for (const pulse of pulses) {
          const age = seconds - pulse.start;
          if (age < 0 || age > CLICK_SECONDS) continue;
          const reach = age * major * 0.55;
          const from = Math.hypot(pulse.x - cx, pulse.y - cy);
          const crossing = Math.abs(reach - Math.abs(radius - from));
          if (crossing < 70)
            wave += (1 - crossing / 70) * (1 - age / CLICK_SECONDS);
        }
        const bright = 0.26 + Math.min(wave, 1) * 0.45;

        context.strokeStyle = color(bright, 0, 6);
        context.lineWidth = ring.style === 'plain' ? 1.2 : 0.8;
        if (ring.style === 'arcs') {
          // Arcos con huecos: tres tramos de longitud distinta.
          context.lineWidth = 2;
          for (let a = 0; a < 3; a += 1) {
            const start = turn + (a / 3) * Math.PI * 2;
            context.beginPath();
            context.arc(
              cx,
              cy,
              radius,
              start,
              start + Math.PI * (0.32 + a * 0.18),
            );
            context.stroke();
          }
        } else {
          context.beginPath();
          context.arc(cx, cy, radius, 0, Math.PI * 2);
          context.stroke();
        }

        if (ring.style === 'ticks') {
          // Marcas de escala: cada 6° una corta, cada 30° una larga.
          for (let step = 0; step < 60; step += 1) {
            const angle = turn + (step / 60) * Math.PI * 2;
            const long = step % 5 === 0;
            const inner = radius - (long ? 16 : 7);
            context.strokeStyle = color(
              long ? bright + 0.1 : bright * 0.8,
              0,
              10,
            );
            context.lineWidth = long ? 1.3 : 0.8;
            context.beginPath();
            context.moveTo(
              cx + Math.cos(angle) * inner,
              cy + Math.sin(angle) * inner,
            );
            context.lineTo(
              cx + Math.cos(angle) * radius,
              cy + Math.sin(angle) * radius,
            );
            context.stroke();
          }
        }

        if (ring.style === 'dots') {
          for (let step = 0; step < 48; step += 1) {
            const angle = turn + (step / 48) * Math.PI * 2;
            context.fillStyle = color(bright + 0.08, 0, 12);
            context.beginPath();
            context.arc(
              cx + Math.cos(angle) * radius,
              cy + Math.sin(angle) * radius,
              1.6,
              0,
              Math.PI * 2,
            );
            context.fill();
          }
        }

        // Nodos brillantes que viajan con el anillo, unidos al centro por un hilo.
        for (let n = 0; n < ring.nodes; n += 1) {
          const angle = turn * 1.3 + (n / ring.nodes) * Math.PI * 2 + index;
          const x = cx + Math.cos(angle) * radius;
          const y = cy + Math.sin(angle) * radius;
          if (index % 2 === 1 && n === 0) {
            context.strokeStyle = color(0.07 + wave * 0.2, 0, 4);
            context.lineWidth = 0.8;
            context.beginPath();
            context.moveTo(cx, cy);
            context.lineTo(x, y);
            context.stroke();
          }
          const glow = context.createRadialGradient(x, y, 0, x, y, 12);
          glow.addColorStop(0, color(0.85, 0, 22));
          glow.addColorStop(1, color(0, 0, 22));
          context.fillStyle = glow;
          context.beginPath();
          context.arc(x, y, 12, 0, Math.PI * 2);
          context.fill();
        }
      });

      // Ondas de los clics, como aros que se expanden y se desvanecen.
      for (const pulse of pulses) {
        const age = seconds - pulse.start;
        if (age < 0 || age > CLICK_SECONDS) continue;
        const fade = 1 - age / CLICK_SECONDS;
        context.strokeStyle = color(0.45 * fade, 0, 18);
        context.lineWidth = 1.5;
        context.beginPath();
        context.arc(pulse.x, pulse.y, age * major * 0.55, 0, Math.PI * 2);
        context.stroke();
      }
      context.globalCompositeOperation = 'source-over';
    };

    const tick = (now: number) => {
      const seconds = now / 1000;
      const step = last === 0 ? 0.016 : Math.min((now - last) / 1000, 0.05);
      last = now;
      pointer.sx += (pointer.x - pointer.sx) * 0.05;
      pointer.sy += (pointer.y - pointer.sy) * 0.05;
      for (const ring of rings) {
        // Resorte hacia la marca a la que encajó, con un pequeño rebote.
        ring.snap += (ring.snapTarget - ring.snap) * Math.min(1, step * 7);
      }
      for (let i = pulses.length - 1; i >= 0; i -= 1) {
        const pulse = pulses[i];
        if (pulse !== undefined && seconds - pulse.start > CLICK_SECONDS)
          pulses.splice(i, 1);
      }
      draw(seconds);
      frame = window.requestAnimationFrame(tick);
    };

    const onMove = (event: PointerEvent) => {
      pointer.x = event.clientX / Math.max(width, 1);
      pointer.y = event.clientY / Math.max(height, 1);
    };
    const onDown = (event: PointerEvent) => {
      pulses.push({
        x: event.clientX,
        y: event.clientY,
        start: performance.now() / 1000,
      });
      // El anillo cuyo radio queda más cerca del clic encaja en su marca siguiente.
      const major = Math.max(width, height) * 0.72;
      const from = Math.hypot(
        event.clientX - width * 0.72,
        event.clientY - height * 0.5,
      );
      let best: Ring | undefined;
      let gap = Infinity;
      for (const ring of rings) {
        const distance = Math.abs(ring.radius * major - from);
        if (distance < gap) {
          gap = distance;
          best = ring;
        }
      }
      if (best !== undefined)
        best.snapTarget += DETENT * Math.sign(best.speed || 1);
    };

    resize();
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    let stopScroll: (() => void) | null = null;
    if (reduced) {
      const onScroll = () => {
        dirty = true;
      };
      scroller?.addEventListener('scroll', onScroll, { passive: true });
      window.addEventListener('scroll', onScroll, { passive: true });
      stopScroll = () => {
        scroller?.removeEventListener('scroll', onScroll);
        window.removeEventListener('scroll', onScroll);
      };
      const redraw = () => {
        if (dirty) {
          draw(0);
          dirty = false;
        }
        frame = window.requestAnimationFrame(redraw);
      };
      frame = window.requestAnimationFrame(redraw);
    } else {
      window.addEventListener('pointermove', onMove, { passive: true });
      window.addEventListener('pointerdown', onDown, { passive: true });
      frame = window.requestAnimationFrame(tick);
    }

    return () => {
      window.cancelAnimationFrame(frame);
      stopScroll?.();
      resizeObserver.disconnect();
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerdown', onDown);
    };
  }, [preset]);

  return (
    <div className="instrument-stage" aria-hidden="true">
      <canvas ref={canvasRef} className="instrument-canvas" />
    </div>
  );
}
