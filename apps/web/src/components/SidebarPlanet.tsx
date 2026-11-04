import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import { auroraTone, type AuroraPreset } from './AuroraBackground';
import { buildPlanets, drawPlanets, stepPlanets, type Planet } from './planets';

/**
 * Planeta de nodos que ocupa el hueco libre de la barra lateral, entre la
 * navegación y los comandos, para que no se vea vacía junto al resto del diseño.
 * Gira solo, toma el color del ambiente y no recibe clics.
 */
export function SidebarPlanet({ preset }: { preset: AuroraPreset }) {
  const holderRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const holder = holderRef.current;
    const canvas = canvasRef.current;
    if (
      holder === null ||
      canvas === null ||
      typeof window.matchMedia !== 'function'
    )
      return;
    const context = canvas.getContext('2d');
    if (context === null) return;

    const reduced = prefersReducedMotion();
    const planet: Planet | undefined = buildPlanets(auroraTone(preset))[0];
    if (planet === undefined) return;
    planet.yaw = 0.6;

    let width = 0;
    let height = 0;
    let frame = 0;
    let last = 0;

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      width = holder.clientWidth;
      height = holder.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
      planet.cx = width / 2;
      planet.cy = height / 2;
      planet.radius = Math.min(
        Math.max(Math.min(width, height) * 0.27, 24),
        66,
      );
      draw(0);
    };

    const draw = (seconds: number) => {
      context.clearRect(0, 0, width, height);
      // Si el hueco es muy bajo (ventana corta), no se dibuja nada.
      if (height < 90) return;
      drawPlanets(context, [planet], seconds, reduced);
    };

    const tick = (now: number) => {
      const step = last === 0 ? 0.016 : Math.min((now - last) / 1000, 0.05);
      last = now;
      stepPlanets([planet], step);
      draw(now / 1000);
      frame = window.requestAnimationFrame(tick);
    };

    const observer = new ResizeObserver(resize);
    observer.observe(holder);
    resize();
    if (!reduced) frame = window.requestAnimationFrame(tick);

    return () => {
      window.cancelAnimationFrame(frame);
      observer.disconnect();
    };
  }, [preset]);

  return (
    <div ref={holderRef} className="sidebar-planet" aria-hidden="true">
      <canvas ref={canvasRef} />
    </div>
  );
}
