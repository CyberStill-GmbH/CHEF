import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';
import { auroraTone, type AuroraPreset } from './AuroraBackground';

type Vec = readonly [number, number, number];

const SURFACE_POINTS = 2600;
const NODE_COUNT = 120;
const SHORT_LINK_ANGLE = 0.42;
const MAX_SHORT_DEGREE = 3;
const LONG_ARC_MIN = 0.5;
const LONG_ARC_MAX = 0.95;
const MAX_ARCS = 34;
const PACKET_COUNT = 10;
const ARC_STEPS = 28;
const STAR_COUNT = 150;

interface Link {
  a: Vec;
  b: Vec;
  omega: number;
  /** Cuánto se eleva el arco sobre la superficie (0 = pegado a ella). */
  lift: number;
}

interface Packet {
  link: number;
  progress: number;
  speed: number;
}

interface Texel {
  point: Vec;
  /** 0 = océano oscuro … 3 = tierra muy brillante. */
  level: number;
}

interface Star {
  x: number;
  y: number;
  size: number;
  phase: number;
}

/** Generador pseudoaleatorio con semilla: el paisaje es siempre el mismo. */
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

function hash3(x: number, y: number, z: number): number {
  const value = Math.sin(x * 127.1 + y * 311.7 + z * 74.7) * 43758.5453;
  return value - Math.floor(value);
}

/** Ruido de valor 3D suave, para dar relieve de "continentes" a la superficie. */
function noise3(x: number, y: number, z: number): number {
  const ix = Math.floor(x);
  const iy = Math.floor(y);
  const iz = Math.floor(z);
  const fx = x - ix;
  const fy = y - iy;
  const fz = z - iz;
  const ux = fx * fx * (3 - 2 * fx);
  const uy = fy * fy * (3 - 2 * fy);
  const uz = fz * fz * (3 - 2 * fz);
  const lerp = (a: number, b: number, t: number) => a + (b - a) * t;
  const corner = (dx: number, dy: number, dz: number) =>
    hash3(ix + dx, iy + dy, iz + dz);
  return lerp(
    lerp(
      lerp(corner(0, 0, 0), corner(1, 0, 0), ux),
      lerp(corner(0, 1, 0), corner(1, 1, 0), ux),
      uy,
    ),
    lerp(
      lerp(corner(0, 0, 1), corner(1, 0, 1), ux),
      lerp(corner(0, 1, 1), corner(1, 1, 1), ux),
      uy,
    ),
    uz,
  );
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

function makeLink(a: Vec, b: Vec, lift: number): Link {
  return { a, b, omega: Math.acos(Math.min(1, dot(a, b))), lift };
}

/** Malla fina entre nodos vecinos, pegada a la superficie. */
function buildMesh(nodes: readonly Vec[]): Link[] {
  const links: Link[] = [];
  const degree = new Array<number>(nodes.length).fill(0);
  const threshold = Math.cos(SHORT_LINK_ANGLE);
  nodes.forEach((a, i) => {
    nodes.forEach((b, j) => {
      if (j <= i) return;
      if (dot(a, b) < threshold) return;
      if (
        (degree[i] ?? 0) >= MAX_SHORT_DEGREE ||
        (degree[j] ?? 0) >= MAX_SHORT_DEGREE
      ) {
        return;
      }
      degree[i] = (degree[i] ?? 0) + 1;
      degree[j] = (degree[j] ?? 0) + 1;
      links.push(makeLink(a, b, 0));
    });
  });
  return links;
}

/** Arcos largos que vuelan alto sobre la superficie. */
function buildArcs(nodes: readonly Vec[], random: () => number): Link[] {
  const arcs: Link[] = [];
  let guard = 0;
  while (arcs.length < MAX_ARCS && guard < 4000) {
    guard += 1;
    const a = nodes[Math.floor(random() * nodes.length)];
    const b = nodes[Math.floor(random() * nodes.length)];
    if (a === undefined || b === undefined || a === b) continue;
    const angle = Math.acos(Math.min(1, dot(a, b)));
    if (angle < LONG_ARC_MIN || angle > LONG_ARC_MAX) continue;
    arcs.push(makeLink(a, b, 0.16 + angle * 0.28));
  }
  return arcs;
}

/** Punto de un arco sobre la esfera, elevado según su altura. */
function arcPoint(link: Link, t: number): Vec {
  if (link.omega < 1e-6) return link.a;
  const sine = Math.sin(link.omega);
  const wa = Math.sin((1 - t) * link.omega) / sine;
  const wb = Math.sin(t * link.omega) / sine;
  const lift = 1 + link.lift * Math.sin(Math.PI * t);
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

/** Un punto no se ve si queda detrás de la esfera y dentro de su silueta. */
function isVisible(point: Vec): boolean {
  return point[2] >= 0 || point[0] * point[0] + point[1] * point[1] >= 1;
}

function buildTexels(): Texel[] {
  return fibonacciSphere(SURFACE_POINTS).map((point) => {
    const coarse = noise3(point[0] * 2.4 + 9, point[1] * 2.4, point[2] * 2.4);
    const fine = noise3(point[0] * 7 + 3, point[1] * 7, point[2] * 7 + 5);
    const value = coarse * 0.7 + fine * 0.3;
    const level = value > 0.62 ? 3 : value > 0.54 ? 2 : value > 0.46 ? 1 : 0;
    return { point, level };
  });
}

function buildStars(random: () => number): Star[] {
  return Array.from({ length: STAR_COUNT }, () => ({
    x: random(),
    y: random() * 0.62,
    size: random() < 0.12 ? 2 : 1,
    phase: random() * Math.PI * 2,
  }));
}

/**
 * Horizonte planetario minimalista: un tramo de la superficie de un planeta
 * con atmósfera brillante, estrellas, una malla fina de nodos y arcos de luz
 * que vuelan sobre ella. Es un canvas propio (proyección ortográfica de una
 * esfera), sin dependencias. Aparece al llegar al final de la página, la
 * superficie gira despacio, se inclina un poco hacia el cursor y toma el color
 * del ambiente activo. Solo dibuja mientras está a la vista y respeta
 * prefers-reduced-motion (queda como una imagen fija).
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
    const random = seeded(7);
    const texels = buildTexels();
    const nodes = fibonacciSphere(NODE_COUNT);
    const mesh = buildMesh(nodes);
    const arcs = buildArcs(nodes, random);
    const stars = buildStars(random);
    const packets: Packet[] = Array.from(
      { length: Math.min(PACKET_COUNT, arcs.length) },
      (_, index) => ({
        link: (index * 3) % Math.max(arcs.length, 1),
        progress: (index * 0.29) % 1,
        speed: 0.00016 + (index % 4) * 0.00005,
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
      const hsl = (alpha: number, lightnessShift = 0, hueShift = 0) =>
        `hsla(${tone.hue + hueShift}, ${tone.saturation}%, ${Math.max(
          4,
          Math.min(94, tone.lightness + lightnessShift),
        )}%, ${alpha})`;

      // El progreso se acota a [0, 1]: el primer fotograma puede llegar con una
      // marca de tiempo anterior a la del instante de revelado, y un valor
      // negativo daría radios negativos (el canvas lanzaría un error).
      const progress = Math.min(Math.max((time - revealedAt) / 1600, 0), 1);
      const eased = reduced ? 1 : 1 - Math.pow(1 - progress, 3);

      const radius = Math.max(width * 0.72, height * 1.25);
      const centerX = width / 2;
      const top = height * 0.4;
      const centerY = top + radius + (1 - eased) * height * 0.3;
      const yaw = reduced ? 0.9 : time * 0.00011 + pointer.sx * 0.25;
      const pitch = 1.12 + pointer.sy * 0.05;

      context.clearRect(0, 0, width, height);
      context.globalCompositeOperation = 'source-over';

      // Estrellas.
      for (const star of stars) {
        const twinkle = reduced
          ? 0.6
          : 0.35 + 0.5 * (0.5 + 0.5 * Math.sin(time * 0.0014 + star.phase));
        context.fillStyle = hsl(twinkle * eased, 38, 0);
        context.fillRect(star.x * width, star.y * height, star.size, star.size);
      }

      // Atmósfera detrás del planeta.
      const atmosphere = context.createRadialGradient(
        centerX,
        centerY,
        radius * 0.98,
        centerX,
        centerY,
        radius * 1.14,
      );
      atmosphere.addColorStop(0, hsl(0.55 * eased, 4));
      atmosphere.addColorStop(0.3, hsl(0.18 * eased, 0));
      atmosphere.addColorStop(1, hsl(0));
      context.fillStyle = atmosphere;
      context.fillRect(0, 0, width, height);

      // Cuerpo del planeta: más claro arriba (luz rasante), casi negro abajo.
      const body = context.createLinearGradient(0, top, 0, top + height * 0.55);
      body.addColorStop(0, hsl(0.9, -34));
      body.addColorStop(0.35, 'rgba(10, 7, 5, 0.97)');
      body.addColorStop(1, 'rgba(5, 4, 3, 1)');
      context.fillStyle = body;
      context.beginPath();
      context.arc(centerX, centerY, radius, 0, Math.PI * 2);
      context.fill();

      // Superficie: miles de puntos con relieve de continentes.
      const buckets: number[][] = [[], [], [], []];
      for (const texel of texels) {
        const rotated = rotate(texel.point, yaw, pitch);
        if (rotated[2] <= 0) continue;
        const x = centerX + rotated[0] * radius;
        const y = centerY - rotated[1] * radius;
        if (x < -4 || x > width + 4 || y < top - 6 || y > height + 4) continue;
        buckets[texel.level]?.push(x, y);
      }
      const levels = [0.16, 0.34, 0.58, 0.9];
      buckets.forEach((coordinates, level) => {
        context.fillStyle = hsl(
          (levels[level] ?? 0.1) * eased,
          level * 9,
          level === 3 ? 8 : 0,
        );
        const size = level === 3 ? 2.1 : 1.6;
        for (let index = 0; index < coordinates.length; index += 2) {
          context.fillRect(
            coordinates[index] ?? 0,
            coordinates[index + 1] ?? 0,
            size,
            size,
          );
        }
      });

      context.globalCompositeOperation = 'lighter';

      // Malla fina sobre la superficie.
      context.lineWidth = 1;
      context.strokeStyle = hsl(0.2 * eased, 6);
      context.beginPath();
      for (const link of mesh) {
        const a = rotate(link.a, yaw, pitch);
        const b = rotate(link.b, yaw, pitch);
        if (a[2] <= 0 || b[2] <= 0) continue;
        context.moveTo(centerX + a[0] * radius, centerY - a[1] * radius);
        context.lineTo(centerX + b[0] * radius, centerY - b[1] * radius);
      }
      context.stroke();

      // Arcos de luz que vuelan sobre la superficie.
      context.lineWidth = 1.25;
      for (const arc of arcs) {
        const points: Vec[] = [];
        for (let step = 0; step <= ARC_STEPS; step += 1) {
          points.push(rotate(arcPoint(arc, step / ARC_STEPS), yaw, pitch));
        }
        context.strokeStyle = hsl(0.72 * eased, 24);
        context.beginPath();
        let drawing = false;
        for (const point of points) {
          const x = centerX + point[0] * radius;
          const y = centerY - point[1] * radius;
          if (isVisible(point) && y > top - radius * 0.5) {
            if (drawing) context.lineTo(x, y);
            else context.moveTo(x, y);
            drawing = true;
          } else {
            drawing = false;
          }
        }
        context.stroke();
      }

      // Nodos con su resplandor; algunos laten.
      nodes.forEach((node, index) => {
        const point = rotate(node, yaw, pitch);
        if (point[2] <= 0) return;
        const x = centerX + point[0] * radius;
        const y = centerY - point[1] * radius;
        if (x < -10 || x > width + 10 || y < top - 10 || y > height + 10)
          return;
        const glow = context.createRadialGradient(x, y, 0, x, y, 9);
        glow.addColorStop(0, hsl(0.9 * eased, 30));
        glow.addColorStop(1, hsl(0));
        context.fillStyle = glow;
        context.beginPath();
        context.arc(x, y, 9, 0, Math.PI * 2);
        context.fill();
        context.fillStyle = hsl(eased, 34);
        context.beginPath();
        context.arc(x, y, 1.7, 0, Math.PI * 2);
        context.fill();
        if (!reduced && index % 6 === 0) {
          const pulse = (time / 1800 + index * 0.17) % 1;
          context.strokeStyle = hsl((1 - pulse) * 0.5, 20);
          context.lineWidth = 1;
          context.beginPath();
          context.arc(x, y, 3 + pulse * 15, 0, Math.PI * 2);
          context.stroke();
        }
      });

      // Paquetes de luz que viajan por los arcos.
      for (const packet of packets) {
        const arc = arcs[packet.link];
        if (arc === undefined) continue;
        const point = rotate(arcPoint(arc, packet.progress), yaw, pitch);
        if (!isVisible(point)) continue;
        const x = centerX + point[0] * radius;
        const y = centerY - point[1] * radius;
        if (y < top - radius * 0.5 || y > height + 12) continue;
        const glow = context.createRadialGradient(x, y, 0, x, y, 11);
        glow.addColorStop(0, hsl(eased, 38));
        glow.addColorStop(1, hsl(0));
        context.fillStyle = glow;
        context.beginPath();
        context.arc(x, y, 11, 0, Math.PI * 2);
        context.fill();
      }

      // Borde brillante del planeta (el "limbo" iluminado).
      context.lineWidth = 6;
      context.strokeStyle = hsl(0.16 * eased, 6);
      context.beginPath();
      context.arc(centerX, centerY, radius, 0, Math.PI * 2);
      context.stroke();
      context.lineWidth = 1.6;
      context.strokeStyle = hsl(0.95 * eased, 22);
      context.beginPath();
      context.arc(centerX, centerY, radius, 0, Math.PI * 2);
      context.stroke();

      context.globalCompositeOperation = 'source-over';
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
            Math.max(arcs.length, 1);
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
    if (!reduced) {
      root.addEventListener('pointermove', onPointer, { passive: true });
    }

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
