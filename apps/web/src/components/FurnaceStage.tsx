import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import {
  auroraChroma,
  auroraTone,
  type AuroraPreset,
} from './AuroraBackground';
import { useScrollProgress } from './scrollProgress';

/** Matiz de Dune: las llamas se definen como desfases respecto a él. */
const DUNE_HUE = 26;

const FLAME_LAYERS = [
  // Del fondo al frente: más altas y rojas atrás, más bajas y claras delante.
  {
    count: 20,
    width: 0.075,
    height: 1.0,
    hue: 8 - DUNE_HUE,
    sat: 96,
    light: 36,
    alpha: 0.2,
  },
  {
    count: 26,
    width: 0.052,
    height: 0.78,
    hue: 20 - DUNE_HUE,
    sat: 100,
    light: 48,
    alpha: 0.17,
  },
  {
    count: 20,
    width: 0.036,
    height: 0.52,
    hue: 33 - DUNE_HUE,
    sat: 100,
    light: 58,
    alpha: 0.14,
  },
] as const;
const WISP_COUNT = 16;
const EMBER_MAX = 190;
/** Radio (px) alrededor del cursor dentro del cual las llamas se apartan. */
const AVOID_RADIUS = 170;

interface Tongue {
  x: number;
  width: number;
  height: number;
  phase: number;
  speed: number;
}

interface Wisp {
  x: number;
  height: number;
  phase: number;
  amplitude: number;
  speed: number;
}

interface Ember {
  x: number;
  y: number;
  vx: number;
  vy: number;
  size: number;
  age: number;
  life: number;
  hue: number;
  phase: number;
}

interface Point {
  x: number;
  y: number;
}

/** Generador con semilla: la forma de las llamas es siempre la misma. */
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
 * Horno: un fondo oscuro que se calienta al bajar por la página. Arriba solo
 * hay brasas dispersas; abajo, un resplandor rojizo y llamas que crecen con el
 * avance del scroll. Es un canvas 2D propio, sin dependencias. Las llamas y las
 * brasas se apartan del cursor. Con "reducir movimiento" queda una imagen fija.
 */
export function FurnaceStage({
  progressRef,
  preset,
}: {
  /** Ambiente activo: de él salen los colores de las llamas. */
  preset: AuroraPreset;
  /** 0 = arriba de la página … 1 = al final (lo escribe FurnaceSpacer). */
  progressRef: { current: number };
}) {
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const sparksRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    const sparksCanvas = sparksRef.current;
    if (
      canvas === null ||
      sparksCanvas === null ||
      typeof window.matchMedia !== 'function'
    )
      return;
    const context = canvas.getContext('2d');
    const spark = sparksCanvas.getContext('2d');
    if (context === null || spark === null) return;

    const reduced = prefersReducedMotion();
    const random = seeded(11);
    // Colores del ambiente: Dune da brasa y ámbar; los demás desplazan el matiz y
    // Silk (croma casi 0) queda en blancos y grises, algo más claros para verse.
    const tone = auroraTone(preset);
    const chroma = auroraChroma(preset);
    const lift = Math.max(0, tone.lightness - 55) * 0.6;
    const wrap = (offset: number) => (tone.hue + offset + 360) % 360;
    const layers = FLAME_LAYERS.map((layer) => ({
      ...layer,
      hue: wrap(layer.hue),
      sat: layer.sat * chroma,
      light: layer.light + lift,
      tongues: Array.from({ length: layer.count }, (_, index): Tongue => ({
        x: (index + 0.15 + random() * 0.7) / layer.count,
        width: layer.width * (0.7 + random() * 0.7),
        height: layer.height * (0.55 + random() * 0.65),
        phase: random() * Math.PI * 2,
        speed: 0.7 + random() * 0.8,
      })),
    }));
    const wisps = Array.from({ length: WISP_COUNT }, (): Wisp => ({
      x: random(),
      height: 0.25 + random() * 0.5,
      phase: random() * Math.PI * 2,
      amplitude: 26 + random() * 44,
      speed: 0.6 + random() * 0.9,
    }));
    const embers: Ember[] = [];
    const pointer = { x: -9999, y: -9999, sx: -9999, sy: -9999 };

    let width = 0;
    let height = 0;
    let frame = 0;
    let last = 0;
    let drawnProgress = -1;
    let dirty = true;
    let spawnDebt = 0;

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 1.5);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      sparksCanvas.width = canvas.width;
      sparksCanvas.height = canvas.height;
      spark.setTransform(ratio, 0, 0, ratio, 0, 0);
      dirty = true;
    };

    /** Aparta un punto del cursor, con una caída suave hacia el borde. */
    const avoid = (point: Point): Point => {
      const dx = point.x - pointer.sx;
      const dy = point.y - pointer.sy;
      const distance = Math.hypot(dx, dy);
      if (distance >= AVOID_RADIUS || distance < 0.001) return point;
      const falloff = 1 - distance / AVOID_RADIUS;
      const push = falloff * falloff * AVOID_RADIUS * 0.6;
      return {
        x: point.x + (dx / distance) * push,
        y: point.y + (dy / distance) * push * 0.7,
      };
    };

    const spawnEmber = (heat: number, anywhere: boolean) => {
      if (embers.length >= EMBER_MAX) return;
      const fromBottom = !anywhere;
      embers.push({
        x: random() * width,
        y: fromBottom
          ? height - random() * height * (0.05 + 0.35 * heat)
          : random() * height,
        vx: (random() - 0.5) * 14,
        vy: -(10 + random() * 34) * (1 + heat * 1.4),
        size: 0.8 + random() * (1.4 + heat * 1.4),
        age: 0,
        life: 2.4 + random() * 4.2,
        hue: wrap(10 - DUNE_HUE + random() * 34),
        phase: random() * Math.PI * 2,
      });
    };

    const stepEmbers = (seconds: number, heat: number) => {
      // Arriba y con poco calor solo hay brasas sueltas por toda la pantalla;
      // al calentarse nacen cada vez más cerca de las llamas.
      spawnDebt += (5 + 85 * heat) * seconds;
      while (spawnDebt >= 1) {
        spawnDebt -= 1;
        spawnEmber(heat, random() > heat * 1.1);
      }
      for (let index = embers.length - 1; index >= 0; index -= 1) {
        const ember = embers[index];
        if (ember === undefined) continue;
        ember.age += seconds;
        if (ember.age >= ember.life || ember.y < -10) {
          embers.splice(index, 1);
          continue;
        }
        const dx = ember.x - pointer.sx;
        const dy = ember.y - pointer.sy;
        const distance = Math.hypot(dx, dy);
        if (distance < AVOID_RADIUS && distance > 0.001) {
          const push = (1 - distance / AVOID_RADIUS) * 260 * seconds;
          ember.x += (dx / distance) * push;
          ember.y += (dy / distance) * push;
        }
        ember.x +=
          (ember.vx + Math.sin(ember.age * 2.1 + ember.phase) * 9) * seconds;
        ember.y += ember.vy * seconds;
      }
    };

    const draw = (time: number) => {
      const progress = Math.min(Math.max(progressRef.current, 0), 1);
      const heat = progress * progress * (3 - 2 * progress);
      const glowAlpha = 0.06 + 0.94 * heat;
      const flameHeight = height * (0.05 + 0.5 * heat);

      context.clearRect(0, 0, width, height);
      context.globalCompositeOperation = 'source-over';

      // Resplandor de la brasa en la parte baja de la pantalla.
      const glow = context.createLinearGradient(0, height, 0, height * 0.25);
      glow.addColorStop(
        0,
        `hsla(${wrap(10 - DUNE_HUE)}, ${100 * chroma}%, ${40 + lift}%, ${0.62 * glowAlpha})`,
      );
      glow.addColorStop(
        0.35,
        `hsla(${wrap(8 - DUNE_HUE)}, ${95 * chroma}%, ${26 + lift}%, ${0.3 * glowAlpha})`,
      );
      glow.addColorStop(
        1,
        `hsla(${wrap(6 - DUNE_HUE)}, ${90 * chroma}%, ${14 + lift * 0.5}%, 0)`,
      );
      context.fillStyle = glow;
      context.fillRect(0, 0, width, height);

      context.globalCompositeOperation = 'lighter';

      // Lenguas de fuego: cada capa es un único trazo con un degradado vertical.
      for (const layer of layers) {
        const reach = flameHeight * layer.height;
        const gradient = context.createLinearGradient(
          0,
          height,
          0,
          height - reach * 1.15,
        );
        const a = layer.alpha * (0.35 + 0.65 * heat);
        gradient.addColorStop(
          0,
          `hsla(${layer.hue + 10}, ${layer.sat}%, ${layer.light + 12}%, ${a * 1.5})`,
        );
        gradient.addColorStop(
          0.55,
          `hsla(${layer.hue}, ${layer.sat}%, ${layer.light}%, ${a})`,
        );
        gradient.addColorStop(
          1,
          `hsla(${layer.hue - 4}, ${layer.sat}%, ${layer.light - 14}%, 0)`,
        );
        context.fillStyle = gradient;
        context.beginPath();
        for (const tongue of layer.tongues) {
          const baseX = tongue.x * width;
          const half = tongue.width * width * (0.5 + 0.5 * heat);
          const sway =
            Math.sin(time * 0.0011 * tongue.speed + tongue.phase) * half * 0.9 +
            Math.sin(time * 0.0027 * tongue.speed + tongue.phase * 2) *
              half *
              0.4;
          const flicker =
            0.82 +
            0.18 * Math.sin(time * 0.004 * tongue.speed + tongue.phase * 3);
          const lift = reach * tongue.height * flicker;
          const tip = avoid({ x: baseX + sway, y: height - lift });
          const left = avoid({
            x: baseX - half * 0.8 + sway * 0.4,
            y: height - lift * 0.55,
          });
          const right = avoid({
            x: baseX + half * 0.9 + sway * 0.5,
            y: height - lift * 0.5,
          });
          context.moveTo(baseX - half, height + 2);
          context.quadraticCurveTo(left.x, left.y, tip.x, tip.y);
          context.quadraticCurveTo(right.x, right.y, baseX + half, height + 2);
          context.closePath();
        }
        context.fill();
      }

      context.globalCompositeOperation = 'source-over';

      spark.clearRect(0, 0, width, height);
      spark.globalCompositeOperation = 'lighter';

      // Hilos de llama: volutas finas que se enroscan al subir.
      spark.lineCap = 'round';
      spark.lineWidth = 1.6;
      spark.strokeStyle = `hsla(${wrap(0)}, ${100 * chroma}%, ${60 + lift}%, ${0.05 + 0.2 * heat})`;
      spark.beginPath();
      for (const wisp of wisps) {
        const baseX = wisp.x * width;
        const reach = flameHeight * 1.15 * wisp.height;
        const points: Point[] = [];
        for (let step = 0; step <= 12; step += 1) {
          const t = step / 12;
          points.push(
            avoid({
              x:
                baseX +
                Math.sin(t * 7.5 + time * 0.0016 * wisp.speed + wisp.phase) *
                  wisp.amplitude *
                  (0.25 + t),
              y: height - reach * t,
            }),
          );
        }
        // Curva suave por los puntos medios, sin esquinas.
        const first = points[0];
        if (first === undefined) continue;
        spark.moveTo(first.x, first.y);
        for (let index = 1; index < points.length - 1; index += 1) {
          const current = points[index];
          const next = points[index + 1];
          if (current === undefined || next === undefined) continue;
          spark.quadraticCurveTo(
            current.x,
            current.y,
            (current.x + next.x) / 2,
            (current.y + next.y) / 2,
          );
        }
      }
      spark.stroke();

      // Brasas: puntos de luz que suben y titilan.
      for (const ember of embers) {
        const fade =
          Math.min(ember.age / 0.5, 1) * (1 - ember.age / ember.life);
        const flicker = 0.65 + 0.35 * Math.sin(time * 0.012 + ember.phase * 5);
        const alpha = Math.max(fade, 0) * flicker;
        spark.fillStyle = `hsla(${ember.hue}, ${100 * chroma}%, ${56 + ember.size * 5 + lift}%, ${alpha})`;
        spark.beginPath();
        spark.arc(ember.x, ember.y, ember.size, 0, Math.PI * 2);
        spark.fill();
        if (ember.size > 1.9) {
          spark.fillStyle = `hsla(${ember.hue}, ${100 * chroma}%, ${50 + lift}%, ${alpha * 0.18})`;
          spark.beginPath();
          spark.arc(ember.x, ember.y, ember.size * 4, 0, Math.PI * 2);
          spark.fill();
        }
      }

      spark.globalCompositeOperation = 'source-over';
    };

    const tick = (now: number) => {
      const seconds = last === 0 ? 0.016 : Math.min((now - last) / 1000, 0.05);
      last = now;
      const progress = Math.min(Math.max(progressRef.current, 0), 1);
      const heat = progress * progress * (3 - 2 * progress);
      pointer.sx += (pointer.x - pointer.sx) * 0.18;
      pointer.sy += (pointer.y - pointer.sy) * 0.18;
      stepEmbers(seconds, heat);
      draw(now);
      frame = window.requestAnimationFrame(tick);
    };

    const onPointer = (event: PointerEvent) => {
      pointer.x = event.clientX;
      pointer.y = event.clientY;
      if (pointer.sx < -999) {
        pointer.sx = pointer.x;
        pointer.sy = pointer.y;
      }
    };
    const onLeave = () => {
      pointer.x = -9999;
      pointer.y = -9999;
    };

    resize();
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);

    if (reduced) {
      // Imagen fija: brasas repartidas y llamas a su altura actual.
      for (let index = 0; index < 70; index += 1) {
        spawnEmber(0.4, random() > 0.5);
        const ember = embers[embers.length - 1];
        if (ember !== undefined) ember.age = ember.life * random() * 0.7;
      }
      const redraw = () => {
        if (dirty || progressRef.current !== drawnProgress) {
          draw(0);
          drawnProgress = progressRef.current;
          dirty = false;
        }
        frame = window.requestAnimationFrame(redraw);
      };
      frame = window.requestAnimationFrame(redraw);
    } else {
      window.addEventListener('pointermove', onPointer, { passive: true });
      document.addEventListener('pointerleave', onLeave);
      frame = window.requestAnimationFrame(tick);
    }

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      window.removeEventListener('pointermove', onPointer);
      document.removeEventListener('pointerleave', onLeave);
    };
  }, [progressRef, preset]);

  return (
    <div className="furnace-stage" aria-hidden="true">
      <canvas ref={canvasRef} className="furnace-canvas furnace-flames" />
      <canvas ref={sparksRef} className="furnace-canvas furnace-sparks" />
    </div>
  );
}

/**
 * Espacio al final de Inicio: deja libre la franja donde el horno está más
 * encendido para que el brillo no tape el texto, y calcula el avance del scroll
 * (0 arriba, 1 abajo) que gobierna el calor del fondo.
 */
export function FurnaceSpacer({
  progressRef,
}: {
  progressRef: { current: number };
}) {
  const ref = useRef<HTMLDivElement>(null);
  useScrollProgress(ref, progressRef);

  return <div ref={ref} className="furnace-spacer" aria-hidden="true" />;
}
