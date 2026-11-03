import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import { auroraTone, type AuroraPreset } from './AuroraBackground';

type Vec = readonly [number, number, number];

const NODE_COUNT = 54;
const LINK_ANGLE = 0.62;
const MAX_DEGREE = 3;
const PACKET_COUNT = 7;
const ARC_STEPS = 14;

interface Link {
  a: Vec;
  b: Vec;
  omega: number;
}

interface Packet {
  link: number;
  progress: number;
  speed: number;
}

/** Puntos repartidos de forma casi uniforme sobre una esfera unitaria. */
function fibonacciSphere(count: number): Vec[] {
  const points: Vec[] = [];
  const golden = Math.PI * (3 - Math.sqrt(5));
  for (let index = 0; index < count; index += 1) {
    const y = 1 - (index / (count - 1)) * 2;
    const ring = Math.sqrt(1 - y * y);
    const theta = golden * index;
    points.push([Math.cos(theta) * ring, y, Math.sin(theta) * ring]);
  }
  return points;
}

function dot(a: Vec, b: Vec): number {
  return a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
}

/** Une cada nodo con sus vecinos cercanos, con un grado máximo para no saturar. */
function buildLinks(nodes: readonly Vec[]): Link[] {
  const links: Link[] = [];
  const degree = new Array<number>(nodes.length).fill(0);
  const threshold = Math.cos(LINK_ANGLE);
  nodes.forEach((a, i) => {
    nodes.forEach((b, j) => {
      if (j <= i) return;
      const similarity = dot(a, b);
      if (similarity < threshold) return;
      if ((degree[i] ?? 0) >= MAX_DEGREE || (degree[j] ?? 0) >= MAX_DEGREE) {
        return;
      }
      degree[i] = (degree[i] ?? 0) + 1;
      degree[j] = (degree[j] ?? 0) + 1;
      links.push({ a, b, omega: Math.acos(Math.min(1, similarity)) });
    });
  });
  return links;
}

/** Punto de un arco sobre la esfera, elevado un poco para darle volumen. */
function arcPoint(link: Link, t: number): Vec {
  const sine = Math.sin(link.omega);
  const wa = Math.sin((1 - t) * link.omega) / sine;
  const wb = Math.sin(t * link.omega) / sine;
  const lift = 1 + 0.1 * Math.sin(Math.PI * t);
  return [
    (wa * link.a[0] + wb * link.b[0]) * lift,
    (wa * link.a[1] + wb * link.b[1]) * lift,
    (wa * link.a[2] + wb * link.b[2]) * lift,
  ];
}

function rotate(point: Vec, yaw: number, pitch: number): Vec {
  const cy = Math.cos(yaw);
  const sy = Math.sin(yaw);
  const cp = Math.cos(pitch);
  const sp = Math.sin(pitch);
  const x1 = point[0] * cy + point[2] * sy;
  const z1 = -point[0] * sy + point[2] * cy;
  return [x1, point[1] * cp - z1 * sp, point[1] * sp + z1 * cp];
}

/** Malla de paralelos y meridianos de la esfera. */
function buildWireframe(): Vec[][] {
  const lines: Vec[][] = [];
  const toRadians = Math.PI / 180;
  for (const latitude of [-60, -30, 0, 30, 60]) {
    const line: Vec[] = [];
    const lat = latitude * toRadians;
    for (let longitude = 0; longitude <= 360; longitude += 6) {
      const lon = longitude * toRadians;
      line.push([
        Math.cos(lat) * Math.cos(lon),
        Math.sin(lat),
        Math.cos(lat) * Math.sin(lon),
      ]);
    }
    lines.push(line);
  }
  for (let meridian = 0; meridian < 12; meridian += 1) {
    const lon = meridian * 30 * toRadians;
    const line: Vec[] = [];
    for (let latitude = -90; latitude <= 90; latitude += 6) {
      const lat = latitude * toRadians;
      line.push([
        Math.cos(lat) * Math.cos(lon),
        Math.sin(lat),
        Math.cos(lat) * Math.sin(lon),
      ]);
    }
    lines.push(line);
  }
  return lines;
}

/**
 * Planeta minimalista de malla con nodos, arcos y paquetes de luz. Es un canvas
 * propio (proyección ortográfica de una esfera), sin dependencias: aparece al
 * llegar al final de la página, gira con el tiempo, se inclina un poco hacia el
 * cursor y toma el color del ambiente activo. Solo dibuja mientras está a la
 * vista y respeta prefers-reduced-motion (queda como una imagen fija).
 */
export function GlobeFooter({ preset }: { preset: AuroraPreset }) {
  const rootRef = useRef<HTMLDivElement>(null);
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const toneRef = useRef(auroraTone(preset));
  toneRef.current = auroraTone(preset);

  useEffect(() => {
    const root = rootRef.current;
    const canvas = canvasRef.current;
    if (root === null || canvas === null) return;
    // Si no se puede dibujar (p. ej. jsdom, sin matchMedia ni canvas), el bloque
    // se muestra igual con su texto, sin planeta.
    if (typeof window.matchMedia !== 'function') {
      root.classList.add('is-revealed');
      return;
    }
    const context = canvas.getContext('2d');
    if (context === null) {
      root.classList.add('is-revealed');
      return;
    }

    const reduced = prefersReducedMotion();
    const nodes = fibonacciSphere(NODE_COUNT);
    const links = buildLinks(nodes);
    const wireframe = buildWireframe();
    const packets: Packet[] = Array.from(
      { length: Math.min(PACKET_COUNT, links.length) },
      (_, index) => ({
        link: (index * 11) % Math.max(links.length, 1),
        progress: (index * 0.37) % 1,
        speed: 0.00022 + (index % 3) * 0.00008,
      }),
    );

    let width = 0;
    let height = 0;
    let frame = 0;
    let visible = false;
    let revealedAt = 0;
    let last = 0;
    const pointer = { x: 0, y: 0, sx: 0, sy: 0 };

    const resize = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 2);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = Math.max(1, Math.round(width * ratio));
      canvas.height = Math.max(1, Math.round(height * ratio));
      context.setTransform(ratio, 0, 0, ratio, 0, 0);
    };

    const draw = (time: number) => {
      const tone = toneRef.current;
      const hsl = (alpha: number, lightnessShift = 0) =>
        `hsla(${tone.hue}, ${tone.saturation}%, ${Math.min(
          92,
          tone.lightness + lightnessShift,
        )}%, ${alpha})`;

      // El progreso se acota a [0, 1]: el primer fotograma puede llegar con una
      // marca de tiempo ligeramente anterior a la del instante de revelado, y un
      // progreso negativo daría un radio negativo (el canvas lanzaría un error).
      const progress = Math.min(Math.max((time - revealedAt) / 1400, 0), 1);
      const grow = reduced ? 1 : 0.7 + 0.3 * (1 - Math.pow(1 - progress, 3));
      const radius = Math.max(0, Math.min(width * 0.46, height * 0.42) * grow);
      const centerX = width / 2;
      const centerY = height * 0.45;
      const yaw = reduced ? 0.7 : time * 0.00011 + pointer.sx * 0.5;
      const pitch = 0.38 + pointer.sy * 0.25;

      context.clearRect(0, 0, width, height);

      const halo = context.createRadialGradient(
        centerX,
        centerY,
        radius * 0.55,
        centerX,
        centerY,
        radius * 1.7,
      );
      halo.addColorStop(0, hsl(0.3));
      halo.addColorStop(1, hsl(0));
      context.fillStyle = halo;
      context.fillRect(0, 0, width, height);

      const body = context.createRadialGradient(
        centerX - radius * 0.3,
        centerY - radius * 0.3,
        radius * 0.1,
        centerX,
        centerY,
        radius,
      );
      body.addColorStop(0, hsl(0.1, 6));
      body.addColorStop(1, hsl(0.01));
      context.fillStyle = body;
      context.beginPath();
      context.arc(centerX, centerY, radius, 0, Math.PI * 2);
      context.fill();

      // Malla: una pasada para la cara visible y otra, más tenue, para la oculta.
      const front = new Path2D();
      const back = new Path2D();
      for (const line of wireframe) {
        let previous: Vec | null = null;
        for (const point of line) {
          const current = rotate(point, yaw, pitch);
          if (previous !== null) {
            const target = previous[2] + current[2] > 0 ? front : back;
            target.moveTo(
              centerX + previous[0] * radius,
              centerY - previous[1] * radius,
            );
            target.lineTo(
              centerX + current[0] * radius,
              centerY - current[1] * radius,
            );
          }
          previous = current;
        }
      }
      context.lineWidth = 1;
      context.strokeStyle = hsl(0.07);
      context.stroke(back);
      context.strokeStyle = hsl(0.5);
      context.stroke(front);

      // Arcos entre nodos vecinos.
      context.lineWidth = 1.1;
      for (const link of links) {
        const points: Vec[] = [];
        for (let step = 0; step <= ARC_STEPS; step += 1) {
          points.push(rotate(arcPoint(link, step / ARC_STEPS), yaw, pitch));
        }
        const middle = points[Math.floor(ARC_STEPS / 2)];
        const depth = middle === undefined ? 0 : middle[2];
        context.strokeStyle = hsl(0.16 + Math.max(depth, 0) * 0.62, 10);
        context.beginPath();
        points.forEach((point, step) => {
          const x = centerX + point[0] * radius;
          const y = centerY - point[1] * radius;
          if (step === 0) context.moveTo(x, y);
          else context.lineTo(x, y);
        });
        context.stroke();
      }

      // Nodos, con un pulso que se expande en algunos.
      nodes.forEach((node, index) => {
        const point = rotate(node, yaw, pitch);
        const x = centerX + point[0] * radius;
        const y = centerY - point[1] * radius;
        const facing = Math.max(point[2], 0);
        context.fillStyle = hsl(0.18 + facing * 0.8, 14);
        context.beginPath();
        context.arc(x, y, 1.6 + facing * 1.8, 0, Math.PI * 2);
        context.fill();
        if (!reduced && index % 7 === 0 && point[2] > 0) {
          const pulse = (time / 1600 + index * 0.13) % 1;
          context.strokeStyle = hsl((1 - pulse) * 0.55 * facing, 10);
          context.beginPath();
          context.arc(x, y, 3 + pulse * 14, 0, Math.PI * 2);
          context.stroke();
        }
      });

      // Paquetes de luz que viajan por los arcos.
      for (const packet of packets) {
        const link = links[packet.link];
        if (link === undefined) continue;
        const point = rotate(arcPoint(link, packet.progress), yaw, pitch);
        const x = centerX + point[0] * radius;
        const y = centerY - point[1] * radius;
        const strength = 0.25 + Math.max(point[2], 0) * 0.75;
        const glow = context.createRadialGradient(x, y, 0, x, y, 9);
        glow.addColorStop(0, hsl(strength, 28));
        glow.addColorStop(1, hsl(0));
        context.fillStyle = glow;
        context.beginPath();
        context.arc(x, y, 9, 0, Math.PI * 2);
        context.fill();
      }
    };

    const tick = (now: number) => {
      const step = last === 0 ? 16 : Math.min(now - last, 64);
      last = now;
      pointer.sx += (pointer.x - pointer.sx) * 0.05;
      pointer.sy += (pointer.y - pointer.sy) * 0.05;
      for (const packet of packets) {
        packet.progress += packet.speed * step;
        if (packet.progress >= 1) {
          packet.progress = 0;
          packet.link =
            (packet.link + 5 + (Math.floor(now) % 7)) %
            Math.max(links.length, 1);
        }
      }
      draw(now);
      frame = visible ? window.requestAnimationFrame(tick) : 0;
    };

    const start = () => {
      if (frame !== 0 || reduced) return;
      last = 0;
      frame = window.requestAnimationFrame(tick);
    };

    const observer =
      typeof IntersectionObserver === 'undefined'
        ? null
        : new IntersectionObserver(
            (entries) => {
              const entry = entries[0];
              if (entry === undefined) return;
              visible = entry.isIntersecting;
              if (visible) {
                root.classList.add('is-revealed');
                if (revealedAt === 0) revealedAt = performance.now();
                if (reduced) draw(performance.now());
                else start();
              }
            },
            { threshold: 0.2 },
          );

    const onPointer = (event: PointerEvent) => {
      const rect = root.getBoundingClientRect();
      pointer.x = ((event.clientX - rect.left) / rect.width - 0.5) * 2;
      pointer.y = ((event.clientY - rect.top) / rect.height - 0.5) * 2;
    };

    resize();
    const resizeObserver = new ResizeObserver(() => {
      resize();
      if (reduced || !visible) draw(performance.now());
    });
    resizeObserver.observe(canvas);
    if (observer === null) {
      visible = true;
      root.classList.add('is-revealed');
      revealedAt = performance.now();
      if (reduced) draw(revealedAt);
      else start();
    } else {
      observer.observe(root);
    }
    if (!reduced)
      root.addEventListener('pointermove', onPointer, { passive: true });

    return () => {
      visible = false;
      window.cancelAnimationFrame(frame);
      observer?.disconnect();
      resizeObserver.disconnect();
      root.removeEventListener('pointermove', onPointer);
    };
  }, []);

  return (
    <div
      ref={rootRef}
      className="globe-footer"
      role="region"
      aria-labelledby="globe-title"
    >
      <canvas ref={canvasRef} className="globe-canvas" aria-hidden="true" />
      <div className="globe-copy">
        <p className="globe-kicker">Red de evidencia</p>
        <h2 id="globe-title">Todo conectado.</h2>
        <p>
          Cada activo es un nodo; cada relación, una arista con su evidencia
          detrás.
        </p>
      </div>
    </div>
  );
}
