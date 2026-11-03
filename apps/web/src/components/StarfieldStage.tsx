import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import {
  buildPlanets,
  drawPlanets,
  layoutPlanets,
  planetAt,
  stepPlanets,
  type Planet,
} from './planets';

const STAR_COUNT = 900;
/** Radio (px) alrededor del cursor dentro del cual las estrellas se apartan. */
const AVOID_RADIUS = 150;
/** Radio (px) y duración (s) del brillo que deja un clic. */
const CLICK_RADIUS = 190;
const CLICK_SECONDS = 1.4;
/** Qué tan rápido fluye el campo hacia afuera (profundidad por segundo). */
const FLOW = 0.018;
const SPIN = 0.006;
/** Elementos sobre los que un clic no agarra un planeta. */
const CONTROLS =
  'button, a, input, select, textarea, summary, label, .project-sidebar, .topbar';

interface Star {
  /** Posición en el plano, entre -1 y 1, antes de la perspectiva. */
  x: number;
  y: number;
  /** Profundidad: 1 = lejos, 0 = encima del observador. */
  z: number;
  size: number;
  phase: number;
  speed: number;
  tint: 0 | 1 | 2;
  flare: boolean;
}

interface Pulse {
  x: number;
  y: number;
  start: number;
}

/** Generador con semilla: el cielo es siempre el mismo al empezar. */
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

/** Disco de luz con núcleo brillante y halo suave, ya dibujado una sola vez. */
function makeSprite(rgb: readonly [number, number, number]): HTMLCanvasElement {
  const size = 64;
  const sprite = document.createElement('canvas');
  sprite.width = size;
  sprite.height = size;
  const context = sprite.getContext('2d');
  if (context === null) return sprite;
  const [r, g, b] = rgb;
  const gradient = context.createRadialGradient(
    size / 2,
    size / 2,
    0,
    size / 2,
    size / 2,
    size / 2,
  );
  gradient.addColorStop(0, `rgba(${r}, ${g}, ${b}, 1)`);
  gradient.addColorStop(0.12, `rgba(${r}, ${g}, ${b}, 0.95)`);
  gradient.addColorStop(0.3, `rgba(${r}, ${g}, ${b}, 0.32)`);
  gradient.addColorStop(1, `rgba(${r}, ${g}, ${b}, 0)`);
  context.fillStyle = gradient;
  context.fillRect(0, 0, size, size);
  return sprite;
}

/**
 * Cielo estrellado para Evidencia: estrellas redondas con un halo suave que
 * fluyen despacio desde el centro, titilan, se apartan del cursor y brillan más,
 * solo cerca del punto, cuando se hace clic. Es un canvas 2D propio, sin
 * dependencias. Con "reducir movimiento" queda una imagen fija.
 */
export function StarfieldStage() {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (canvas === null || typeof window.matchMedia !== 'function') return;
    const context = canvas.getContext('2d');
    if (context === null) return;

    const reduced = prefersReducedMotion();
    const random = seeded(31);
    const sprites = [
      makeSprite([236, 240, 255]),
      makeSprite([255, 214, 176]),
      makeSprite([186, 212, 255]),
    ] as const;
    const stars: Star[] = Array.from({ length: STAR_COUNT }, () => {
      const pick = random();
      return {
        x: (random() * 2 - 1) * 1.6,
        y: (random() * 2 - 1) * 1.6,
        z: 0.08 + random() * 0.92,
        size: 0.8 + Math.pow(random(), 2.2) * 3,
        phase: random() * Math.PI * 2,
        speed: 0.5 + random() * 1.6,
        tint: pick < 0.68 ? 0 : pick < 0.88 ? 1 : 2,
        flare: random() < 0.05,
      };
    });
    const pulses: Pulse[] = [];
    const planets = buildPlanets();
    let grab: {
      planet: Planet;
      x: number;
      y: number;
      time: number;
    } | null = null;
    const overControl = (target: EventTarget | null) =>
      target instanceof Element && target.closest(CONTROLS) !== null;
    const pointer = { x: -9999, y: -9999, sx: -9999, sy: -9999 };

    let width = 0;
    let height = 0;
    let frame = 0;
    let last = 0;
    let angle = 0;
    let dirty = true;

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      layoutPlanets(planets, width, height);
      dirty = true;
    };

    const draw = (seconds: number) => {
      context.clearRect(0, 0, width, height);
      context.globalCompositeOperation = 'lighter';
      const centerX = width / 2;
      const centerY = height / 2;
      const reach = Math.max(width, height) * 0.62;
      const cos = Math.cos(angle);
      const sin = Math.sin(angle);
      for (const star of stars) {
        const rx = star.x * cos - star.y * sin;
        const ry = star.x * sin + star.y * cos;
        const perspective = 1 / Math.max(star.z, 0.05);
        let x = centerX + rx * perspective * reach * 0.5;
        let y = centerY + ry * perspective * reach * 0.5;
        if (x < -40 || x > width + 40 || y < -40 || y > height + 40) continue;

        // Se aparta del cursor con una caída suave hacia el borde.
        const dx = x - pointer.sx;
        const dy = y - pointer.sy;
        const away = Math.hypot(dx, dy);
        if (away < AVOID_RADIUS && away > 0.01) {
          const falloff = 1 - away / AVOID_RADIUS;
          const push = falloff * falloff * AVOID_RADIUS * 0.55;
          x += (dx / away) * push;
          y += (dy / away) * push;
        }

        const near = 1 - star.z;
        const twinkle = reduced
          ? 0.85
          : 0.7 + 0.3 * Math.sin(seconds * star.speed + star.phase);
        const arrive = Math.min(1, (1 - star.z) * 6);
        let boost = 0;
        for (const pulse of pulses) {
          const age = seconds - pulse.start;
          if (age < 0 || age > CLICK_SECONDS) continue;
          const distance = Math.hypot(x - pulse.x, y - pulse.y);
          if (distance >= CLICK_RADIUS) continue;
          boost +=
            Math.pow(1 - distance / CLICK_RADIUS, 1.5) *
            Math.exp(-age * 2.6) *
            2.2;
        }
        const diameter = (star.size * (0.8 + near * 1.5) + boost * 5) * 3.2;
        context.globalAlpha = Math.min(
          1,
          (0.5 + near * 0.5) * twinkle * arrive + boost * 0.5,
        );
        context.drawImage(
          sprites[star.tint],
          x - diameter / 2,
          y - diameter / 2,
          diameter,
          diameter,
        );
        if (star.flare && near > 0.4) {
          // Destellos finos en cruz para las estrellas más brillantes.
          const length = diameter * 2.4;
          context.globalAlpha = Math.min(0.5, 0.22 * twinkle + boost * 0.2);
          context.fillStyle = '#eef0ff';
          context.fillRect(x - length / 2, y - 0.4, length, 0.8);
          context.fillRect(x - 0.4, y - length / 2, 0.8, length);
        }
      }
      context.globalAlpha = 1;
      context.globalCompositeOperation = 'source-over';
      drawPlanets(context, planets, seconds, reduced);
    };

    const tick = (now: number) => {
      const seconds = now / 1000;
      const step = last === 0 ? 0.016 : Math.min((now - last) / 1000, 0.05);
      last = now;
      pointer.sx += (pointer.x - pointer.sx) * 0.16;
      pointer.sy += (pointer.y - pointer.sy) * 0.16;
      angle += SPIN * step;
      stepPlanets(planets, step);
      for (const star of stars) {
        star.z -= FLOW * step * (0.4 + (1 - star.z) * 1.6);
        if (star.z < 0.06) {
          // La estrella llegó al observador: reaparece lejos, en otro sitio.
          star.z = 1;
          star.x = (random() * 2 - 1) * 1.6;
          star.y = (random() * 2 - 1) * 1.6;
        }
      }
      for (let index = pulses.length - 1; index >= 0; index -= 1) {
        const pulse = pulses[index];
        if (pulse !== undefined && seconds - pulse.start > CLICK_SECONDS) {
          pulses.splice(index, 1);
        }
      }
      draw(seconds);
      frame = window.requestAnimationFrame(tick);
    };

    const onPointer = (event: PointerEvent) => {
      pointer.x = event.clientX;
      pointer.y = event.clientY;
      if (pointer.sx < -999) {
        pointer.sx = pointer.x;
        pointer.sy = pointer.y;
      }
      if (grab !== null) {
        // Girar el planeta agarrado según lo que se mueve el puntero.
        const now = performance.now();
        const seconds = Math.max((now - grab.time) / 1000, 0.008);
        const dx = event.clientX - grab.x;
        const dy = event.clientY - grab.y;
        grab.planet.yaw += dx * 0.012;
        grab.planet.pitch = Math.min(
          Math.max(grab.planet.pitch + dy * 0.012, -1.3),
          1.3,
        );
        grab.planet.yawSpeed = Math.max(
          -7,
          Math.min(7, (dx * 0.012) / seconds),
        );
        grab.planet.pitchSpeed = Math.max(
          -5,
          Math.min(5, (dy * 0.012) / seconds),
        );
        grab.x = event.clientX;
        grab.y = event.clientY;
        grab.time = now;
        return;
      }
      const hover =
        !overControl(event.target) &&
        planetAt(planets, event.clientX, event.clientY) !== null;
      document.documentElement.style.cursor = hover ? 'grab' : '';
    };
    const onDown = (event: PointerEvent) => {
      pulses.push({
        x: event.clientX,
        y: event.clientY,
        start: performance.now() / 1000,
      });
      if (overControl(event.target)) return;
      const planet = planetAt(planets, event.clientX, event.clientY);
      if (planet === null) return;
      planet.grabbed = true;
      planet.yawSpeed = 0;
      planet.pitchSpeed = 0;
      grab = {
        planet,
        x: event.clientX,
        y: event.clientY,
        time: performance.now(),
      };
      document.documentElement.style.cursor = 'grabbing';
      document.body.style.userSelect = 'none';
    };
    const onUp = () => {
      if (grab === null) return;
      grab.planet.grabbed = false;
      grab = null;
      document.documentElement.style.cursor = '';
      document.body.style.userSelect = '';
    };
    const onLeave = () => {
      pointer.x = -9999;
      pointer.y = -9999;
    };

    resize();
    const resizeObserver = new ResizeObserver(resize);
    resizeObserver.observe(canvas);
    if (reduced) {
      const redraw = () => {
        if (dirty) {
          draw(0);
          dirty = false;
        }
        frame = window.requestAnimationFrame(redraw);
      };
      frame = window.requestAnimationFrame(redraw);
    } else {
      window.addEventListener('pointermove', onPointer, { passive: true });
      window.addEventListener('pointerdown', onDown, { passive: true });
      window.addEventListener('pointerup', onUp, { passive: true });
      window.addEventListener('pointercancel', onUp, { passive: true });
      document.addEventListener('pointerleave', onLeave);
      frame = window.requestAnimationFrame(tick);
    }

    return () => {
      window.cancelAnimationFrame(frame);
      resizeObserver.disconnect();
      window.removeEventListener('pointermove', onPointer);
      window.removeEventListener('pointerdown', onDown);
      window.removeEventListener('pointerup', onUp);
      window.removeEventListener('pointercancel', onUp);
      document.documentElement.style.cursor = '';
      document.body.style.userSelect = '';
      document.removeEventListener('pointerleave', onLeave);
    };
  }, []);

  return (
    <div className="starfield-stage" aria-hidden="true">
      <canvas ref={canvasRef} className="starfield-canvas" />
    </div>
  );
}
