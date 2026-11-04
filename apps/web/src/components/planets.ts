/**
 * Planetas de nodos para el cielo de Evidencia: cada uno es una malla de puntos
 * de luz unidos por líneas finas, con su propio carácter (mundo con continentes,
 * gigante con anillos, bandas, caras grandes y una luna). Giran despacio solos y
 * se pueden agarrar y girar con el puntero. Dibujo en canvas 2D con proyección
 * ortográfica; sin dependencias.
 */

type Vec3 = readonly [number, number, number];

type PlanetStyle = 'earth' | 'ringed' | 'banded' | 'facets' | 'moon';

interface PlanetSpec {
  style: PlanetStyle;
  /** Posición horizontal (fracción del ancho) y vertical (fracción del recorrido del fondo). */
  x: number;
  y: number;
  /** Cuánto de lo que se desplaza la página se mueve este planeta (0 = fijo). */
  parallax: number;
  /** Radio como fracción del lado menor de la pantalla. */
  size: number;
  hue: number;
  saturation: number;
  tilt: number;
  spin: number;
}

interface PlanetNode {
  point: Vec3;
  /** 0 = tenue … 1 = muy brillante. */
  weight: number;
  /** Los nodos de anillo no quedan tapados por la esfera salvo que estén detrás. */
  ring: boolean;
}

export interface Planet {
  spec: PlanetSpec;
  nodes: PlanetNode[];
  links: Array<readonly [number, number]>;
  sprite: HTMLCanvasElement;
  cx: number;
  cy: number;
  radius: number;
  yaw: number;
  pitch: number;
  yawSpeed: number;
  pitchSpeed: number;
  grabbed: boolean;
}

/** Plano de cada planeta: el matiz sale del ambiente activo más un desfase. */
type PlanetBlueprint = Omit<PlanetSpec, 'hue' | 'saturation'> & {
  hueOffset: number;
  satScale: number;
};

const SPECS: readonly PlanetBlueprint[] = [
  {
    style: 'earth',
    x: 0.84,
    y: 0.1,
    parallax: 0.5,
    size: 0.095,
    hueOffset: 2,
    satScale: 1,
    tilt: 0.35,
    spin: 0.16,
  },
  {
    style: 'ringed',
    x: 0.2,
    y: 0.34,
    parallax: 0.4,
    size: 0.075,
    hueOffset: 10,
    satScale: 1,
    tilt: 0.5,
    spin: -0.12,
  },
  {
    style: 'banded',
    x: 0.8,
    y: 0.55,
    parallax: 0.55,
    size: 0.058,
    hueOffset: -12,
    satScale: 1,
    tilt: 0.25,
    spin: 0.22,
  },
  {
    style: 'facets',
    x: 0.3,
    y: 0.76,
    parallax: 0.35,
    size: 0.05,
    hueOffset: -4,
    satScale: 1,
    tilt: 0.6,
    spin: -0.18,
  },
  {
    style: 'moon',
    x: 0.88,
    y: 0.94,
    parallax: 0.6,
    size: 0.036,
    hueOffset: 6,
    satScale: 0.4,
    tilt: 0.2,
    spin: 0.1,
  },
];

export function hslToRgb(
  h: number,
  s: number,
  l: number,
): [number, number, number] {
  const a = (s / 100) * Math.min(l / 100, 1 - l / 100);
  const f = (n: number) => {
    const k = (n + h / 30) % 12;
    return Math.round(
      255 * (l / 100 - a * Math.max(-1, Math.min(k - 3, Math.min(9 - k, 1)))),
    );
  };
  return [f(0), f(8), f(4)];
}

function makeSprite(spec: PlanetSpec): HTMLCanvasElement {
  const size = 48;
  const sprite = document.createElement('canvas');
  sprite.width = size;
  sprite.height = size;
  const context = sprite.getContext('2d');
  if (context === null) return sprite;
  const [r, g, b] = hslToRgb(spec.hue, spec.saturation, 66);
  const gradient = context.createRadialGradient(24, 24, 0, 24, 24, 24);
  gradient.addColorStop(0, 'rgba(255, 255, 255, 1)');
  gradient.addColorStop(0.16, `rgba(${r}, ${g}, ${b}, 0.95)`);
  gradient.addColorStop(0.4, `rgba(${r}, ${g}, ${b}, 0.28)`);
  gradient.addColorStop(1, `rgba(${r}, ${g}, ${b}, 0)`);
  context.fillStyle = gradient;
  context.fillRect(0, 0, size, size);
  return sprite;
}

function fibonacci(count: number): Vec3[] {
  const golden = Math.PI * (3 - Math.sqrt(5));
  return Array.from({ length: count }, (_, index) => {
    const y = 1 - ((index + 0.5) / count) * 2;
    const ring = Math.sqrt(1 - y * y);
    const theta = golden * index;
    return [Math.cos(theta) * ring, y, Math.sin(theta) * ring] as const;
  });
}

function hash(x: number, y: number, z: number): number {
  const value = Math.sin(x * 127.1 + y * 311.7 + z * 74.7) * 43758.5453;
  return value - Math.floor(value);
}

function smoothNoise(x: number, y: number, z: number): number {
  const ix = Math.floor(x);
  const iy = Math.floor(y);
  const iz = Math.floor(z);
  const fx = x - ix;
  const fy = y - iy;
  const fz = z - iz;
  const u = fx * fx * (3 - 2 * fx);
  const v = fy * fy * (3 - 2 * fy);
  const w = fz * fz * (3 - 2 * fz);
  const mix = (a: number, b: number, t: number) => a + (b - a) * t;
  const c = (dx: number, dy: number, dz: number) =>
    hash(ix + dx, iy + dy, iz + dz);
  return mix(
    mix(mix(c(0, 0, 0), c(1, 0, 0), u), mix(c(0, 1, 0), c(1, 1, 0), u), v),
    mix(mix(c(0, 0, 1), c(1, 0, 1), u), mix(c(0, 1, 1), c(1, 1, 1), u), v),
    w,
  );
}

/** Une cada nodo con sus `k` vecinos más cercanos (sin repetir pares). */
function nearestLinks(
  points: readonly Vec3[],
  from: number,
  to: number,
  k: number,
  maxDistance: number,
): Array<readonly [number, number]> {
  const links: Array<readonly [number, number]> = [];
  const seen = new Set<number>();
  for (let i = from; i < to; i += 1) {
    const a = points[i];
    if (a === undefined) continue;
    const near: Array<{ index: number; distance: number }> = [];
    for (let j = from; j < to; j += 1) {
      const b = points[j];
      if (j === i || b === undefined) continue;
      const distance = Math.hypot(a[0] - b[0], a[1] - b[1], a[2] - b[2]);
      if (distance <= maxDistance) near.push({ index: j, distance });
    }
    near.sort((p, q) => p.distance - q.distance);
    for (const { index } of near.slice(0, k)) {
      const key = Math.min(i, index) * 10000 + Math.max(i, index);
      if (seen.has(key)) continue;
      seen.add(key);
      links.push([i, index]);
    }
  }
  return links;
}

function buildGeometry(spec: PlanetSpec): {
  nodes: PlanetNode[];
  links: Array<readonly [number, number]>;
} {
  const nodes: PlanetNode[] = [];
  let links: Array<readonly [number, number]> = [];
  const add = (point: Vec3, weight: number, ring = false) => {
    nodes.push({ point, weight, ring });
  };

  if (spec.style === 'earth') {
    for (const point of fibonacci(190)) {
      const land =
        0.7 * smoothNoise(point[0] * 2.3 + 9, point[1] * 2.3, point[2] * 2.3) +
        0.3 * smoothNoise(point[0] * 6, point[1] * 6 + 4, point[2] * 6);
      add(point, land > 0.52 ? 1 : 0.28);
    }
    links = nearestLinks(
      nodes.map((node) => node.point),
      0,
      nodes.length,
      3,
      0.36,
    );
  } else if (spec.style === 'ringed') {
    for (const point of fibonacci(96)) add(point, 0.55);
    links = nearestLinks(
      nodes.map((node) => node.point),
      0,
      nodes.length,
      3,
      0.5,
    );
    const tilt = 0.32;
    for (const [radius, count, weight] of [
      [1.7, 64, 0.85],
      [2.1, 52, 0.5],
    ] as const) {
      const start = nodes.length;
      for (let i = 0; i < count; i += 1) {
        const angle = (i / count) * Math.PI * 2;
        const x = Math.cos(angle) * radius;
        const z = Math.sin(angle) * radius;
        add([x, -z * Math.sin(tilt), z * Math.cos(tilt)], weight, true);
      }
      for (let i = 0; i < count; i += 1) {
        links.push([start + i, start + ((i + 1) % count)]);
      }
    }
  } else if (spec.style === 'banded') {
    const bands = 9;
    const bandStart: number[] = [];
    const bandCount: number[] = [];
    for (let band = 0; band < bands; band += 1) {
      const lat = (-0.5 + (band + 0.5) / bands) * Math.PI * 0.94;
      const count = Math.max(6, Math.round(22 * Math.cos(lat)));
      bandStart.push(nodes.length);
      bandCount.push(count);
      for (let i = 0; i < count; i += 1) {
        const lon = (i / count) * Math.PI * 2 + band * 0.4;
        add(
          [
            Math.cos(lat) * Math.cos(lon),
            Math.sin(lat),
            Math.cos(lat) * Math.sin(lon),
          ],
          band % 2 === 0 ? 1 : 0.45,
        );
      }
    }
    for (let band = 0; band < bands; band += 1) {
      const start = bandStart[band] ?? 0;
      const count = bandCount[band] ?? 0;
      for (let i = 0; i < count; i += 1) {
        links.push([start + i, start + ((i + 1) % count)]);
        const nextStart = bandStart[band + 1];
        const nextCount = bandCount[band + 1];
        if (nextStart !== undefined && nextCount !== undefined) {
          links.push([
            start + i,
            nextStart + Math.floor((i / count) * nextCount),
          ]);
        }
      }
    }
  } else if (spec.style === 'facets') {
    for (const point of fibonacci(42)) add(point, 0.8);
    links = nearestLinks(
      nodes.map((node) => node.point),
      0,
      nodes.length,
      4,
      0.9,
    );
  } else {
    for (const point of fibonacci(84)) {
      add(
        point,
        hash(point[0] * 9, point[1] * 9, point[2] * 9) > 0.92 ? 1 : 0.4,
      );
    }
    links = nearestLinks(
      nodes.map((node) => node.point),
      0,
      nodes.length,
      2,
      0.42,
    );
  }
  return { nodes, links };
}

export interface AmbientTone {
  hue: number;
  saturation: number;
}

export function buildPlanets(tone: AmbientTone): Planet[] {
  return SPECS.map((blueprint, index) => {
    const { hueOffset, satScale, ...rest } = blueprint;
    const spec: PlanetSpec = {
      ...rest,
      hue: (tone.hue + hueOffset + 360) % 360,
      saturation: tone.saturation * satScale,
    };
    const { nodes, links } = buildGeometry(spec);
    return {
      spec,
      nodes,
      links,
      sprite: makeSprite(spec),
      cx: 0,
      cy: 0,
      radius: 40,
      yaw: index * 1.7,
      pitch: spec.tilt,
      yawSpeed: 0,
      pitchSpeed: 0,
      grabbed: false,
    };
  });
}

export function layoutPlanets(
  planets: readonly Planet[],
  width: number,
  height: number,
) {
  const base = Math.min(width, height);
  for (const planet of planets) {
    planet.cx = planet.spec.x * width;
    planet.radius = Math.min(Math.max(planet.spec.size * base, 22), 120);
  }
}

/**
 * Coloca cada planeta según el scroll: el fondo se desliza más despacio que la
 * página (paralaje), así que al bajar van apareciendo planetas nuevos. Las
 * posiciones verticales se reparten por todo el recorrido del fondo.
 */
export function placePlanets(
  planets: readonly Planet[],
  height: number,
  scrollTop: number,
  maxScroll: number,
) {
  const reach = height + 0.6 * maxScroll;
  for (const planet of planets) {
    planet.cy = planet.spec.y * reach - scrollTop * planet.spec.parallax;
  }
}

/** Planeta bajo el punto (el más cercano al frente), o null. */
export function planetAt(
  planets: readonly Planet[],
  x: number,
  y: number,
): Planet | null {
  let found: Planet | null = null;
  let best = Infinity;
  for (const planet of planets) {
    const distance = Math.hypot(x - planet.cx, y - planet.cy);
    if (distance <= planet.radius * 1.18 && distance < best) {
      found = planet;
      best = distance;
    }
  }
  return found;
}

/** Giro propio y por inercia; el planeta vuelve despacio a su inclinación. */
export function stepPlanets(planets: readonly Planet[], seconds: number) {
  for (const planet of planets) {
    if (planet.grabbed) continue;
    planet.yaw += (planet.spec.spin + planet.yawSpeed) * seconds;
    planet.pitch += planet.pitchSpeed * seconds;
    planet.pitch += (planet.spec.tilt - planet.pitch) * seconds * 0.35;
    const decay = Math.exp(-seconds * 1.7);
    planet.yawSpeed *= decay;
    planet.pitchSpeed *= decay;
  }
}

export function drawPlanets(
  context: CanvasRenderingContext2D,
  planets: readonly Planet[],
  seconds: number,
  still: boolean,
) {
  for (const planet of planets) {
    const { cx, cy, radius, spec } = planet;
    if (cy < -radius * 2.6 || cy > context.canvas.clientHeight + radius * 2.6)
      continue;
    const hue = spec.hue;
    const sat = spec.saturation;

    // Cuerpo oscuro: tapa las estrellas que quedan detrás del planeta.
    context.globalCompositeOperation = 'source-over';
    const body = context.createRadialGradient(
      cx - radius * 0.3,
      cy - radius * 0.3,
      radius * 0.1,
      cx,
      cy,
      radius,
    );
    body.addColorStop(0, `hsla(${hue}, ${sat * 0.5}%, 12%, 0.94)`);
    body.addColorStop(1, 'rgba(3, 4, 7, 0.96)');
    context.fillStyle = body;
    context.beginPath();
    context.arc(cx, cy, radius * 1.01, 0, Math.PI * 2);
    context.fill();

    context.globalCompositeOperation = 'lighter';
    const glow = context.createRadialGradient(
      cx,
      cy,
      radius * 0.85,
      cx,
      cy,
      radius * 1.6,
    );
    glow.addColorStop(0, `hsla(${hue}, ${sat}%, 55%, 0.22)`);
    glow.addColorStop(1, `hsla(${hue}, ${sat}%, 55%, 0)`);
    context.fillStyle = glow;
    context.beginPath();
    context.arc(cx, cy, radius * 1.6, 0, Math.PI * 2);
    context.fill();
    context.strokeStyle = `hsla(${hue}, ${sat}%, 65%, 0.3)`;
    context.lineWidth = 1;
    context.beginPath();
    context.arc(cx, cy, radius, 0, Math.PI * 2);
    context.stroke();

    // Proyección: giro sobre el eje vertical y luego inclinación.
    const cosY = Math.cos(planet.yaw);
    const sinY = Math.sin(planet.yaw);
    const cosP = Math.cos(planet.pitch);
    const sinP = Math.sin(planet.pitch);
    const count = planet.nodes.length;
    const sx = new Float32Array(count);
    const sy = new Float32Array(count);
    const depth = new Float32Array(count);
    const hidden = new Uint8Array(count);
    planet.nodes.forEach((node, index) => {
      const [x, y, z] = node.point;
      const x1 = x * cosY + z * sinY;
      const z1 = -x * sinY + z * cosY;
      const y2 = y * cosP - z1 * sinP;
      const z2 = y * sinP + z1 * cosP;
      sx[index] = cx + x1 * radius;
      sy[index] = cy - y2 * radius;
      depth[index] = z2;
      // Un nodo de anillo detrás de la esfera queda tapado por ella.
      if (node.ring && z2 < 0 && x1 * x1 + y2 * y2 < 1) hidden[index] = 1;
    });

    context.lineWidth = 0.8;
    for (const front of [false, true]) {
      context.strokeStyle = `hsla(${hue}, ${sat}%, 62%, ${front ? 0.5 : 0.13})`;
      context.beginPath();
      for (const [a, b] of planet.links) {
        if (hidden[a] === 1 || hidden[b] === 1) continue;
        const isFront = ((depth[a] ?? 0) + (depth[b] ?? 0)) / 2 > 0;
        if (isFront !== front) continue;
        context.moveTo(sx[a] ?? 0, sy[a] ?? 0);
        context.lineTo(sx[b] ?? 0, sy[b] ?? 0);
      }
      context.stroke();
    }

    const unit = Math.max(radius / 40, 0.6);
    planet.nodes.forEach((node, index) => {
      if (hidden[index] === 1) return;
      const isFront = (depth[index] ?? 0) > 0;
      const twinkle = still ? 1 : 0.82 + 0.18 * Math.sin(seconds * 1.6 + index);
      const size = (3.4 + node.weight * 6.2) * unit * (isFront ? 1 : 0.6);
      context.globalAlpha = Math.min(
        1,
        (isFront ? 0.5 + node.weight * 0.5 : 0.18) * twinkle,
      );
      context.drawImage(
        planet.sprite,
        (sx[index] ?? 0) - size,
        (sy[index] ?? 0) - size,
        size * 2,
        size * 2,
      );
    });
    context.globalAlpha = 1;
  }
  context.globalCompositeOperation = 'source-over';
}
