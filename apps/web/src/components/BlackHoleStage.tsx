import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import {
  advanceTrace,
  buildPalette,
  createTrace,
  FIELD_HEIGHT,
  FIELD_WIDTH,
  paintTrace,
  SHADOW_RADIUS,
  type BlackHoleTrace,
} from './blackHole';
import { useScrollProgress } from './scrollProgress';

const COLUMNS = 700;
const ROWS = 274;
/** Inclinación de la imagen en el plano de la pantalla (25°, en sentido horario). */
const ROLL = (25 * Math.PI) / 180;
const STAR_COUNT = 720;
/** Radio (px) del clic dentro del cual las estrellas se encienden. */
const CLICK_RADIUS = 160;
const CLICK_SECONDS = 1.6;
/** Milisegundos por fotograma dedicados al trazado inicial. */
const TRACE_MS = 10;

interface Star {
  x: number;
  y: number;
  size: number;
  phase: number;
  alpha: number;
  warm: number;
}

interface Pulse {
  x: number;
  y: number;
  start: number;
}

/** La imagen del agujero se calcula una sola vez y se reutiliza. */
let sharedTrace: BlackHoleTrace | null = null;

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
 * Espacio profundo con un agujero negro a la derecha. La imagen del agujero
 * sale de un trazado de rayos real (ver blackHole.ts): sombra, anillo de
 * fotones, disco de acreción con corrimiento Doppler y el arco que da la vuelta
 * por detrás. Crece de 100 % a ~175 % con el scroll. Las estrellas se curvan
 * alrededor del agujero y, al hacer clic, las cercanas al clic brillan más
 * durante un instante. Con "reducir movimiento" queda una imagen fija.
 */
export function BlackHoleStage({
  progressRef,
}: {
  /** 0 = arriba … 1 = al final (lo escribe BlackHoleProbe). */
  progressRef: { current: number };
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null || typeof window.matchMedia !== 'function') return;
    const context = canvas.getContext('2d');
    const holeCanvas = document.createElement('canvas');
    holeCanvas.width = COLUMNS;
    holeCanvas.height = ROWS;
    const holeContext = holeCanvas.getContext('2d');
    if (context === null || holeContext === null) return;

    const reduced = prefersReducedMotion();
    const random = seeded(23);
    const palette = buildPalette();
    const image = holeContext.createImageData(COLUMNS, ROWS);
    sharedTrace ??= createTrace(COLUMNS, ROWS);
    const trace = sharedTrace;
    const sharedWasReady = trace.done >= trace.columns * trace.rows;
    const stars: Star[] = Array.from({ length: STAR_COUNT }, () => ({
      x: random(),
      y: random(),
      size: random() < 0.1 ? 2.4 : 1 + random() * 1.1,
      phase: random() * Math.PI * 2,
      alpha: 0.4 + random() * 0.6,
      warm: random(),
    }));
    const pulses: Pulse[] = [];

    let width = 0;
    let height = 0;
    let frame = 0;
    let tick = 0;
    let dirty = true;
    let drawnProgress = -1;
    let drawnDone = -1;
    let finishedAt: number | null = null;

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 1.5);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      dirty = true;
    };

    const draw = (seconds: number) => {
      const progress = Math.min(Math.max(progressRef.current, 0), 1);
      const eased = progress * progress * (3 - 2 * progress);
      const scale = 1.25 + 0.5 * eased;
      const unit = Math.min(Math.max(height * 0.021, 14), 28) * scale;
      const centerX = width * 0.87;
      const centerY = height * 0.58;
      // El agujero aparece con un fundido cuando termina el trazado.
      const appear =
        finishedAt === null
          ? 0
          : reduced
            ? 1
            : Math.min(1, (seconds - finishedAt) / 1.1);
      const shadow = SHADOW_RADIUS * unit;
      const lensRadius = shadow * 0.8 * appear;

      context.clearRect(0, 0, width, height);
      context.globalCompositeOperation = 'source-over';

      // Estrellas: se desplazan por la lente y las cercanas al clic se encienden.
      for (const star of stars) {
        let x = star.x * width;
        let y = star.y * height;
        const dx = x - centerX;
        const dy = y - centerY;
        const distance = Math.hypot(dx, dy);
        if (distance < shadow * 2.6 && distance > 0.5) {
          const apparent =
            0.5 *
            (distance +
              Math.sqrt(distance * distance + 4 * lensRadius * lensRadius));
          if (apparent < shadow * 0.99) continue;
          x = centerX + (dx / distance) * apparent;
          y = centerY + (dy / distance) * apparent;
        }
        const twinkle = reduced
          ? 0.8
          : 0.6 + 0.4 * Math.sin(seconds * 1.3 + star.phase);
        let boost = 0;
        for (const pulse of pulses) {
          const age = seconds - pulse.start;
          if (age < 0 || age > CLICK_SECONDS) continue;
          const away = Math.hypot(x - pulse.x, y - pulse.y);
          if (away >= CLICK_RADIUS) continue;
          boost +=
            Math.pow(1 - away / CLICK_RADIUS, 1.4) * Math.exp(-age * 2.4) * 1.8;
        }
        const alpha = Math.min(1, star.alpha * twinkle + boost * 0.7);
        const size = star.size + boost * 1.6;
        context.fillStyle =
          star.warm > 0.7
            ? `rgba(255, 214, 170, ${alpha})`
            : `rgba(232, 232, 255, ${alpha})`;
        context.fillRect(x - size / 2, y - size / 2, size, size);
        if (boost > 0.35) {
          const halo = context.createRadialGradient(
            x,
            y,
            0,
            x,
            y,
            9 + boost * 5,
          );
          halo.addColorStop(
            0,
            `rgba(255, 220, 180, ${Math.min(boost * 0.35, 0.6)})`,
          );
          halo.addColorStop(1, 'rgba(255, 220, 180, 0)');
          context.fillStyle = halo;
          context.fillRect(x - 16, y - 16, 32, 32);
        }
      }

      // Sombra del agujero: tapa lo que queda detrás.
      context.globalAlpha = appear;
      context.fillStyle = '#000';
      context.beginPath();
      context.arc(centerX, centerY, shadow * 0.985, 0, Math.PI * 2);
      context.fill();

      // Disco, arco lensado y anillo de fotones: se suman como luz.
      context.globalCompositeOperation = 'lighter';
      const halo = context.createRadialGradient(
        centerX,
        centerY,
        shadow * 0.9,
        centerX,
        centerY,
        shadow * 3.2,
      );
      halo.addColorStop(0, `rgba(130, 56, 28, ${0.24 * appear})`);
      halo.addColorStop(1, 'rgba(130, 56, 28, 0)');
      context.fillStyle = halo;
      context.fillRect(0, 0, width, height);
      context.imageSmoothingEnabled = true;
      context.imageSmoothingQuality = 'high';
      context.globalAlpha = appear;
      context.save();
      context.translate(centerX, centerY);
      context.rotate(ROLL);
      context.drawImage(
        holeCanvas,
        -(FIELD_WIDTH / 2) * unit,
        -(FIELD_HEIGHT / 2) * unit,
        FIELD_WIDTH * unit,
        FIELD_HEIGHT * unit,
      );
      context.restore();
      context.globalAlpha = 1;
      context.globalCompositeOperation = 'source-over';
    };

    const paintHole = (seconds: number) => {
      paintTrace(trace, image, palette, seconds);
      holeContext.putImageData(image, 0, 0);
    };

    const loop = (now: number) => {
      const seconds = now / 1000;
      const total = trace.columns * trace.rows;
      if (trace.done < total) advanceTrace(trace, TRACE_MS);
      if (finishedAt === null && trace.done >= total)
        finishedAt = seconds - (sharedWasReady ? 5 : 0);
      if (reduced) {
        if (
          dirty ||
          progressRef.current !== drawnProgress ||
          trace.done !== drawnDone
        ) {
          if (finishedAt !== null) paintHole(0);
          draw(0);
          drawnProgress = progressRef.current;
          drawnDone = trace.done;
          dirty = false;
        }
      } else {
        tick += 1;
        // El disco gira a ~30 cuadros por segundo; las estrellas, a pantalla completa.
        if (finishedAt !== null && tick % 2 === 0) paintHole(seconds);
        draw(seconds);
        for (let index = pulses.length - 1; index >= 0; index -= 1) {
          const pulse = pulses[index];
          if (pulse !== undefined && seconds - pulse.start > CLICK_SECONDS) {
            pulses.splice(index, 1);
          }
        }
      }
      frame = window.requestAnimationFrame(loop);
    };

    const onPointerDown = (event: PointerEvent) => {
      pulses.push({
        x: event.clientX,
        y: event.clientY,
        start: performance.now() / 1000,
      });
    };

    resize();
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    if (!reduced) window.addEventListener('pointerdown', onPointerDown);
    frame = window.requestAnimationFrame(loop);

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      window.removeEventListener('pointerdown', onPointerDown);
    };
  }, [progressRef]);

  return (
    <div className="blackhole-stage" aria-hidden="true">
      <canvas ref={canvasRef} className="blackhole-canvas" />
    </div>
  );
}

/** Mide el avance del scroll de la página para que el agujero crezca. */
export function BlackHoleProbe({
  progressRef,
}: {
  progressRef: { current: number };
}) {
  const ref = useRef<HTMLDivElement>(null);
  useScrollProgress(ref, progressRef);
  return <div ref={ref} className="blackhole-probe" aria-hidden="true" />;
}
