import { useEffect, useRef } from 'react';

export type AuroraPreset = 'aurora' | 'dune' | 'ocean' | 'dusk';

interface PresetDefinition {
  label: string;
  hues: readonly [number, number, number];
  saturation: number;
  lightness: number;
}

const PRESETS: Record<AuroraPreset, PresetDefinition> = {
  aurora: {
    label: 'Aurora',
    hues: [155, 245, 175],
    saturation: 72,
    lightness: 58,
  },
  dune: { label: 'Dune', hues: [28, 14, 42], saturation: 86, lightness: 55 },
  ocean: {
    label: 'Ocean',
    hues: [205, 190, 225],
    saturation: 80,
    lightness: 56,
  },
  dusk: { label: 'Dusk', hues: [270, 310, 240], saturation: 72, lightness: 62 },
};

export const AURORA_PRESETS = Object.keys(PRESETS) as AuroraPreset[];

export function auroraLabel(preset: AuroraPreset): string {
  return PRESETS[preset].label;
}

interface BlobState {
  x: number;
  y: number;
  vx: number;
  vy: number;
  baseX: number;
  baseY: number;
  size: number;
  follow: number;
  phase: number;
}

const BLOB_SEEDS: ReadonlyArray<
  Pick<BlobState, 'baseX' | 'baseY' | 'size' | 'follow' | 'phase'>
> = [
  { baseX: 0.18, baseY: 0.2, size: 0.62, follow: 0.07, phase: 0 },
  { baseX: 0.82, baseY: 0.26, size: 0.56, follow: 0.12, phase: 2.1 },
  { baseX: 0.5, baseY: 0.88, size: 0.64, follow: 0.05, phase: 4.2 },
  { baseX: 0.88, baseY: 0.82, size: 0.48, follow: 0.1, phase: 1.3 },
];

const STIFFNESS = 0.014;
const DAMPING = 0.9;
const MAX_RIPPLES = 6;

function color(preset: PresetDefinition, hue: number, alpha: number): string {
  return `hsla(${hue.toFixed(1)}, ${preset.saturation}%, ${preset.lightness}%, ${alpha})`;
}

/**
 * Fondo vivo: manchas de color que derivan de forma continua, siguen al
 * cursor con un resorte (con algo de rebote) y reaccionan a cada clic con un
 * splash. Solo usa CSS y requestAnimationFrame; sin dependencias.
 * Respeta prefers-reduced-motion (queda estático) y es decorativo (aria-hidden).
 */
export function AuroraBackground({ preset }: { preset: AuroraPreset }) {
  const rootRef = useRef<HTMLDivElement>(null);
  const blobRefs = useRef<Array<HTMLDivElement | null>>([]);
  const presetRef = useRef(PRESETS[preset]);
  presetRef.current = PRESETS[preset];

  useEffect(() => {
    const root = rootRef.current;
    if (root === null) return;

    // Sin matchMedia (p. ej. jsdom) se trata como movimiento reducido: el fondo
    // queda estático en vez de fallar.
    const reduced =
      typeof window.matchMedia !== 'function' ||
      window.matchMedia('(prefers-reduced-motion: reduce)').matches;
    const blobs: BlobState[] = BLOB_SEEDS.map((seed) => ({
      ...seed,
      x: seed.baseX,
      y: seed.baseY,
      vx: 0,
      vy: 0,
    }));
    const pointer = { x: 0.5, y: 0.5, sx: 0.5, sy: 0.5 };
    let width = window.innerWidth;
    let height = window.innerHeight;
    let frame = 0;
    let last = performance.now();

    const paint = (time: number) => {
      const definition = presetRef.current;
      const reach = Math.max(width, height);
      blobs.forEach((blob, index) => {
        const element = blobRefs.current[index];
        if (element === null || element === undefined) return;
        const diameter = blob.size * reach;
        const hue =
          definition.hues[index % definition.hues.length]! +
          Math.sin(time / 7000 + blob.phase) * 24;
        element.style.width = `${diameter}px`;
        element.style.height = `${diameter}px`;
        element.style.transform = `translate3d(${blob.x * width - diameter / 2}px, ${blob.y * height - diameter / 2}px, 0)`;
        element.style.background = `radial-gradient(closest-side, ${color(definition, hue, 0.34)}, ${color(definition, hue, 0)} 72%)`;
      });
    };

    const tick = (now: number) => {
      const step = Math.min((now - last) / 16.7, 3);
      last = now;
      pointer.sx += (pointer.x - pointer.sx) * 0.06 * step;
      pointer.sy += (pointer.y - pointer.sy) * 0.06 * step;
      blobs.forEach((blob) => {
        const targetX =
          blob.baseX +
          Math.sin(now / 9000 + blob.phase) * 0.07 +
          (pointer.sx - 0.5) * blob.follow * 2;
        const targetY =
          blob.baseY +
          Math.cos(now / 11000 + blob.phase) * 0.07 +
          (pointer.sy - 0.5) * blob.follow * 2;
        blob.vx = (blob.vx + (targetX - blob.x) * STIFFNESS * step) * DAMPING;
        blob.vy = (blob.vy + (targetY - blob.y) * STIFFNESS * step) * DAMPING;
        blob.x += blob.vx * step;
        blob.y += blob.vy * step;
      });
      paint(now);
      frame = window.requestAnimationFrame(tick);
    };

    const onMove = (event: PointerEvent) => {
      pointer.x = event.clientX / width;
      pointer.y = event.clientY / height;
    };

    const onDown = (event: PointerEvent) => {
      if (reduced) return;
      const nx = event.clientX / width;
      const ny = event.clientY / height;
      blobs.forEach((blob) => {
        blob.vx += (nx - blob.x) * 0.05;
        blob.vy += (ny - blob.y) * 0.05;
      });
      while (root.querySelectorAll('.aurora-ripple').length >= MAX_RIPPLES) {
        root.querySelector('.aurora-ripple')?.remove();
      }
      const definition = presetRef.current;
      const hue =
        definition.hues[Math.floor(Math.random() * definition.hues.length)]!;
      const ripple = document.createElement('span');
      ripple.className = 'aurora-ripple';
      ripple.style.left = `${event.clientX}px`;
      ripple.style.top = `${event.clientY}px`;
      ripple.style.background = `radial-gradient(circle, transparent 52%, ${color(definition, hue, 0.55)} 60%, transparent 70%)`;
      root.append(ripple);
      const animation = ripple.animate(
        [
          { transform: 'translate(-50%, -50%) scale(0.2)', opacity: 0.95 },
          { transform: 'translate(-50%, -50%) scale(1)', opacity: 0 },
        ],
        { duration: 1000, easing: 'cubic-bezier(0.2, 0.7, 0.2, 1)' },
      );
      animation.onfinish = () => ripple.remove();
    };

    const onResize = () => {
      width = window.innerWidth;
      height = window.innerHeight;
      if (reduced) paint(0);
    };

    window.addEventListener('resize', onResize);
    if (reduced) {
      paint(0);
    } else {
      window.addEventListener('pointermove', onMove, { passive: true });
      window.addEventListener('pointerdown', onDown, { passive: true });
      frame = window.requestAnimationFrame(tick);
    }

    return () => {
      window.cancelAnimationFrame(frame);
      window.removeEventListener('resize', onResize);
      window.removeEventListener('pointermove', onMove);
      window.removeEventListener('pointerdown', onDown);
    };
  }, []);

  return (
    <div ref={rootRef} className="aurora-bg" aria-hidden="true">
      {BLOB_SEEDS.map((seed, index) => (
        <div
          key={seed.phase}
          ref={(element) => {
            blobRefs.current[index] = element;
          }}
          className="aurora-blob"
        />
      ))}
      <div className="aurora-grid" />
    </div>
  );
}
