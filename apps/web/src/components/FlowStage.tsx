import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import { auroraTone, type AuroraPreset } from './AuroraBackground';

/** Separación entre carriles del circuito (px). */
const LANE = 62;
/** Cuánto del scroll acompaña el fondo (paralaje). */
const PARALLAX = 0.32;
const CLICK_SECONDS = 1.5;
const CLICK_RADIUS = 220;

interface Trace {
  /** Vértices de la ruta, de izquierda a derecha. */
  points: Array<{ x: number; y: number }>;
  /** Longitud acumulada hasta cada vértice. */
  length: number[];
  total: number;
}

interface Packet {
  trace: number;
  distance: number;
  speed: number;
  /** Los paquetes que nacen de un clic desaparecen al terminar. */
  temporary: boolean;
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

/** Ruta con tramos horizontales y desvíos en ángulo, como un bus de circuito. */
function buildTrace(
  random: () => number,
  lane: number,
  width: number,
  worldHeight: number,
): Trace {
  const points = [{ x: -30, y: lane * LANE }];
  let x = -30;
  let y = lane * LANE;
  while (x < width + 30) {
    x += 110 + random() * 330;
    points.push({ x, y });
    if (random() < 0.8) {
      const shift =
        (random() < 0.5 ? -1 : 1) * (1 + Math.floor(random() * 3)) * LANE;
      const next = Math.min(Math.max(y + shift, 0), worldHeight);
      const bevel = Math.abs(next - y);
      if (bevel > 0) {
        // Desvío a 45°: avanza tanto como sube o baja.
        points.push({ x: x + bevel, y: next });
        x += bevel;
        y = next;
      }
    }
  }
  const length = [0];
  for (let i = 1; i < points.length; i += 1) {
    const a = points[i - 1];
    const b = points[i];
    if (a === undefined || b === undefined) continue;
    length.push((length[i - 1] ?? 0) + Math.hypot(b.x - a.x, b.y - a.y));
  }
  return { points, length, total: length[length.length - 1] ?? 0 };
}

function pointAt(trace: Trace, distance: number): { x: number; y: number } {
  const d = Math.min(Math.max(distance, 0), trace.total);
  let low = 0;
  let high = trace.length.length - 1;
  while (low < high - 1) {
    const mid = (low + high) >> 1;
    if ((trace.length[mid] ?? 0) <= d) low = mid;
    else high = mid;
  }
  const a = trace.points[low];
  const b = trace.points[high];
  if (a === undefined || b === undefined) return { x: 0, y: 0 };
  const span = (trace.length[high] ?? 0) - (trace.length[low] ?? 0) || 1;
  const t = (d - (trace.length[low] ?? 0)) / span;
  return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
}

/**
 * Fondo de Fuentes: un circuito de datos. Rutas de luz recorren la pantalla y
 * por ellas viajan paquetes; el fondo acompaña el scroll con paralaje, las rutas
 * cercanas al cursor se encienden y un clic lanza una ráfaga de paquetes por las
 * rutas más próximas. Canvas 2D propio, con el color del ambiente.
 */
export function FlowStage({ preset }: { preset: AuroraPreset }) {
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

    const scroller = document.querySelector<HTMLElement>('.main-content');
    const readScroll = () => {
      if (
        scroller !== null &&
        scroller.scrollHeight - scroller.clientHeight > 1
      )
        return scroller.scrollTop;
      return window.scrollY;
    };

    let traces: Trace[] = [];
    let packets: Packet[] = [];
    const pulses: Pulse[] = [];
    const pointer = { x: -9999, y: -9999, sx: -9999, sy: -9999 };
    let width = 0;
    let height = 0;
    let frame = 0;
    let last = 0;
    let dirty = true;
    const random = seeded(47);

    const rebuild = () => {
      const maxScroll = Math.max(
        0,
        scroller !== null && scroller.scrollHeight - scroller.clientHeight > 1
          ? scroller.scrollHeight - scroller.clientHeight
          : document.documentElement.scrollHeight - window.innerHeight,
      );
      const worldHeight = height + PARALLAX * maxScroll + LANE * 2;
      const lanes = Math.floor(worldHeight / LANE);
      traces = [];
      packets = [];
      for (let lane = 0; lane <= lanes; lane += 1) {
        // No todos los carriles llevan ruta: así queda aire entre ellas.
        if (random() < 0.2) continue;
        const trace = buildTrace(random, lane, width, worldHeight);
        traces.push(trace);
        const count = 1 + Math.floor(random() * 2);
        for (let i = 0; i < count; i += 1) {
          packets.push({
            trace: traces.length - 1,
            distance: random() * trace.total,
            speed: 70 + random() * 190,
            temporary: false,
          });
        }
      }
    };

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      rebuild();
      dirty = true;
    };

    /** Cuánto brilla una ruta por la cercanía del cursor o de un clic. */
    const nearness = (y: number, x: number, seconds: number) => {
      let glow = 0;
      const dx = x - pointer.sx;
      const dy = y - pointer.sy;
      const d = Math.hypot(dx, dy);
      if (d < 170) glow += (1 - d / 170) * 0.55;
      for (const pulse of pulses) {
        const age = seconds - pulse.start;
        if (age < 0 || age > CLICK_SECONDS) continue;
        const distance = Math.hypot(x - pulse.x, y - pulse.y);
        if (distance < CLICK_RADIUS) {
          glow += (1 - distance / CLICK_RADIUS) * Math.exp(-age * 2.2) * 1.4;
        }
      }
      return glow;
    };

    const draw = (seconds: number) => {
      context.clearRect(0, 0, width, height);
      const offset = -readScroll() * PARALLAX;
      context.globalCompositeOperation = 'lighter';
      context.lineJoin = 'round';

      for (const trace of traces) {
        const first = trace.points[0];
        if (first === undefined) continue;
        // Cada ruta se dibuja en tramos para que su brillo siga al cursor.
        for (let i = 1; i < trace.points.length; i += 1) {
          const a = trace.points[i - 1];
          const b = trace.points[i];
          if (a === undefined || b === undefined) continue;
          const ya = a.y + offset;
          const yb = b.y + offset;
          if ((ya < -20 && yb < -20) || (ya > height + 20 && yb > height + 20))
            continue;
          const glow = nearness((ya + yb) / 2, (a.x + b.x) / 2, seconds);
          context.strokeStyle = color(0.2 + Math.min(glow, 1.2) * 0.5, 0, 6);
          context.lineWidth = 1 + Math.min(glow, 1) * 0.8;
          context.beginPath();
          context.moveTo(a.x, ya);
          context.lineTo(b.x, yb);
          context.stroke();
          // Nodos en los vértices: puertos del circuito.
          context.fillStyle = color(0.34 + Math.min(glow, 1) * 0.5, 0, 10);
          context.beginPath();
          context.arc(b.x, yb, 2.2, 0, Math.PI * 2);
          context.fill();
        }
      }

      // Paquetes: un punto brillante con cola corta.
      for (const packet of packets) {
        const trace = traces[packet.trace];
        if (trace === undefined) continue;
        const head = pointAt(trace, packet.distance);
        const headY = head.y + offset;
        if (headY < -30 || headY > height + 30) continue;
        for (let step = 0; step < 7; step += 1) {
          const from = pointAt(trace, packet.distance - step * 9);
          const to = pointAt(trace, packet.distance - (step + 1) * 9);
          context.strokeStyle = color(0.55 * (1 - step / 7), 0, 18);
          context.lineWidth = 2.2 * (1 - step / 9);
          context.beginPath();
          context.moveTo(from.x, from.y + offset);
          context.lineTo(to.x, to.y + offset);
          context.stroke();
        }
        const halo = context.createRadialGradient(
          head.x,
          headY,
          0,
          head.x,
          headY,
          13,
        );
        halo.addColorStop(0, color(0.9, 0, 22));
        halo.addColorStop(1, color(0, 0, 22));
        context.fillStyle = halo;
        context.beginPath();
        context.arc(head.x, headY, 13, 0, Math.PI * 2);
        context.fill();
      }
      context.globalCompositeOperation = 'source-over';
    };

    const burst = (x: number, y: number) => {
      const offset = -readScroll() * PARALLAX;
      // Se lanzan paquetes por las rutas más próximas al clic, desde ese punto.
      const ranked = traces
        .map((trace, index) => {
          let best = Infinity;
          let at = 0;
          for (let d = 0; d <= trace.total; d += 24) {
            const p = pointAt(trace, d);
            const away = Math.hypot(p.x - x, p.y + offset - y);
            if (away < best) {
              best = away;
              at = d;
            }
          }
          return { index, best, at };
        })
        .sort((a, b) => a.best - b.best)
        .slice(0, 5);
      for (const { index, at } of ranked) {
        for (let i = 0; i < 2; i += 1) {
          packets.push({
            trace: index,
            distance: at,
            speed: 260 + random() * 260,
            temporary: true,
          });
        }
      }
    };

    const tick = (now: number) => {
      const seconds = now / 1000;
      const step = last === 0 ? 0.016 : Math.min((now - last) / 1000, 0.05);
      last = now;
      pointer.sx += (pointer.x - pointer.sx) * 0.16;
      pointer.sy += (pointer.y - pointer.sy) * 0.16;
      for (let i = packets.length - 1; i >= 0; i -= 1) {
        const packet = packets[i];
        const trace = packet === undefined ? undefined : traces[packet.trace];
        if (packet === undefined || trace === undefined) continue;
        packet.distance += packet.speed * step;
        if (packet.distance > trace.total + 40) {
          if (packet.temporary) packets.splice(i, 1);
          else packet.distance = -40;
        }
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
      pointer.x = event.clientX;
      pointer.y = event.clientY;
      if (pointer.sx < -999) {
        pointer.sx = pointer.x;
        pointer.sy = pointer.y;
      }
    };
    const onDown = (event: PointerEvent) => {
      pulses.push({
        x: event.clientX,
        y: event.clientY,
        start: performance.now() / 1000,
      });
      burst(event.clientX, event.clientY);
    };
    const onLeave = () => {
      pointer.x = -9999;
      pointer.y = -9999;
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
      document.addEventListener('pointerleave', onLeave);
      frame = window.requestAnimationFrame(tick);
    }

    return () => {
      window.cancelAnimationFrame(frame);
      stopScroll?.();
      resizeObserver.disconnect();
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerdown', onDown);
      document.removeEventListener('pointerleave', onLeave);
    };
  }, [preset]);

  return (
    <div className="flow-stage" aria-hidden="true">
      <canvas ref={canvasRef} className="flow-canvas" />
    </div>
  );
}
