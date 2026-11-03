/**
 * Imagen de un agujero negro de Schwarzschild calculada por trazado de rayos.
 *
 * Unidades: G = c = M = 1 (radio del horizonte = 2, ISCO = 6, sombra ≈ 5,2).
 * Cada píxel del plano de la imagen lanza un rayo hacia atrás desde el
 * observador y se integra la órbita de la luz u'' = -u + 3u² (u = 1/r) en el
 * plano de ese rayo. Cada vez que la órbita cruza el plano del disco de
 * acreción se anota el radio y el azimut del punto: el primer cruce es la imagen
 * directa del disco, el segundo la que da la vuelta por detrás del agujero (el
 * arco que se curva sobre la sombra) y el tercero alimenta el anillo de fotones.
 * La asimetría de brillo sale del corrimiento Doppler de Luminet (1979):
 * 1 + z = (1 + Ω·X·sin θ₀) / √(1 − 3/r), con intensidad ∝ g⁴.
 *
 * El trazado es caro, así que se hace una sola vez por tamaño, por tandas, y
 * queda en memoria: después cada fotograma solo recalcula el brillo del disco al
 * girar, sin volver a integrar ningún rayo.
 */

/** Ancho y alto del plano de la imagen, en unidades de M. */
export const FIELD_WIDTH = 50;
export const FIELD_HEIGHT = 16;
/** Radio de la sombra de un agujero de Schwarzschild: 3√3 M. */
export const SHADOW_RADIUS = 5.196;

const INNER_RADIUS = 3.6;
const OUTER_RADIUS = 34;
const STEP = 0.04;
const MAX_SWEEP = Math.PI * 3.2;
const ORDERS = 3;
/** Inclinación respecto del eje del disco: 88° (casi de canto, como Gargantúa). */
const INCLINATION = (86.5 * Math.PI) / 180;

export interface BlackHoleTrace {
  readonly columns: number;
  readonly rows: number;
  /** Cuántos píxeles ya están calculados (se calculan por filas). */
  done: number;
  /** Para cada cruce con el disco: intensidad base, radio y azimut. */
  readonly intensity: Float32Array[];
  readonly radius: Float32Array[];
  readonly azimuth: Float32Array[];
  /** Índices de los píxeles que tienen al menos un cruce. */
  readonly lit: number[];
  /** Desvanecimiento hacia los bordes del campo, para que no haya cortes. */
  readonly fade: Float32Array;
}

export function createTrace(columns: number, rows: number): BlackHoleTrace {
  const count = columns * rows;
  const make = () =>
    Array.from({ length: ORDERS }, () => new Float32Array(count));
  const fade = new Float32Array(count);
  const smooth = (t: number) => {
    const c = Math.min(Math.max(t, 0), 1);
    return c * c * (3 - 2 * c);
  };
  for (let index = 0; index < count; index += 1) {
    const nx = Math.abs(((index % columns) + 0.5) / columns - 0.5) * 2;
    const ny = Math.abs((Math.floor(index / columns) + 0.5) / rows - 0.5) * 2;
    fade[index] = smooth((1 - nx) / 0.25) * smooth((1 - ny) / 0.22);
  }
  return {
    columns,
    rows,
    done: 0,
    intensity: make(),
    radius: make(),
    azimuth: make(),
    lit: [],
    fade,
  };
}

/**
 * Traza píxeles durante, como mucho, `maxMs` milisegundos (para no bloquear la
 * interfaz); devuelve true cuando ya no queda nada.
 */
export function advanceTrace(trace: BlackHoleTrace, maxMs: number): boolean {
  const total = trace.columns * trace.rows;
  const deadline = performance.now() + maxMs;
  const sinTheta = Math.sin(INCLINATION);
  const cosTheta = Math.cos(INCLINATION);
  let index = trace.done;
  for (; index < total; index += 1) {
    if (index % 128 === 0 && performance.now() > deadline) break;
    const column = index % trace.columns;
    const row = Math.floor(index / trace.columns);
    const x = ((column + 0.5) / trace.columns - 0.5) * FIELD_WIDTH;
    const y = (0.5 - (row + 0.5) / trace.rows) * FIELD_HEIGHT;
    const b = Math.hypot(x, y);
    if (b < 1e-4) continue;

    // Primer cruce con el plano del disco (ángulo orbital, en (0, π]).
    const t = Math.atan2(-cosTheta, (y / b) * sinTheta);
    const first = ((t % Math.PI) + Math.PI) % Math.PI || Math.PI;

    let u = 0;
    let du = 1 / b;
    let phi = 0;
    let order = 0;
    let threshold = first;
    let any = false;
    while (phi < MAX_SWEEP && order < ORDERS) {
      const nextDu = du + (-u + 3 * u * u) * STEP;
      const nextU = u + nextDu * STEP;
      const nextPhi = phi + STEP;
      if (nextU >= 0.5 || nextU <= 0) break; // capturado o escapado
      while (order < ORDERS && nextPhi >= threshold) {
        const fraction = (threshold - phi) / STEP;
        const hitU = u + fraction * (nextU - u);
        const r = 1 / hitU;
        if (r >= INNER_RADIUS && r <= OUTER_RADIUS) {
          // Posición del punto de cruce en el plano del disco.
          const sinPhi = Math.sin(threshold);
          const cosPhi = Math.cos(threshold);
          const px = r * (cosPhi * sinTheta - sinPhi * (y / b) * cosTheta);
          const py = r * sinPhi * (x / b);
          const omega = Math.pow(r, -1.5);
          const g =
            Math.sqrt(Math.max(1 - 3 / r, 0.12)) /
            Math.max(1 + 0.55 * omega * x * sinTheta, 0.3);
          const edge = Math.min(1, (r - INNER_RADIUS) / 1.2 + 0.15);
          // El disco directo llega lejos; las imágenes lensadas se concentran
          // junto a la sombra, y el anillo de fotones (3.er cruce) se refuerza.
          // Rayos casi rectos (la banda del ecuador) conservan el brillo lejos
          // del centro; los muy curvados (el arco sobre la sombra) no.
          const bent = Math.min(
            Math.max((Math.abs(threshold - Math.PI / 2) - 0.08) / 0.5, 0),
            1,
          );
          const slope = order === 0 ? 1.1 + 1.6 * bent : 5;
          const boost =
            (order >= 2 ? 2.2 : 1) * (order === 0 ? 1 + 3.2 * (1 - bent) : 1);
          const emissivity = Math.pow(INNER_RADIUS / r, slope) * edge * boost;
          const slot = order;
          (trace.intensity[slot] as Float32Array)[index] =
            Math.pow(g, 1.2) * emissivity;
          (trace.radius[slot] as Float32Array)[index] = r;
          (trace.azimuth[slot] as Float32Array)[index] = Math.atan2(py, px);
          any = true;
        }
        order += 1;
        threshold = first + order * Math.PI;
      }
      u = nextU;
      du = nextDu;
      phi = nextPhi;
    }
    if (any) trace.lit.push(index);
  }
  trace.done = index;
  return index >= total;
}

/** Tabla de color tipo cuerpo negro: brasa oscura → naranja → blanco cálido. */
export function buildPalette(): Uint8ClampedArray {
  const stops: ReadonlyArray<readonly [number, number, number, number]> = [
    [0, 4, 2, 2],
    [0.2, 66, 22, 12],
    [0.4, 168, 72, 38],
    [0.6, 246, 160, 96],
    [0.8, 255, 222, 168],
    [1, 255, 250, 240],
  ];
  const palette = new Uint8ClampedArray(256 * 3);
  for (let i = 0; i < 256; i += 1) {
    const value = i / 255;
    let k = 0;
    while (k < stops.length - 2 && value > (stops[k + 1]?.[0] ?? 1)) k += 1;
    const a = stops[k];
    const c = stops[k + 1];
    if (a === undefined || c === undefined) continue;
    const span = c[0] - a[0] || 1;
    const f = Math.min(Math.max((value - a[0]) / span, 0), 1);
    palette[i * 3] = a[1] + (c[1] - a[1]) * f;
    palette[i * 3 + 1] = a[2] + (c[2] - a[2]) * f;
    palette[i * 3 + 2] = a[3] + (c[3] - a[3]) * f;
  }
  return palette;
}

const STREAK_SIZE = 1024;

/** Ruido de valor 1D suave y determinista. */
function valueNoise(x: number, seed: number): number {
  const hash = (n: number) => {
    const v = Math.sin(n * 127.1 + seed * 311.7) * 43758.5453;
    return v - Math.floor(v);
  };
  const i = Math.floor(x);
  const f = x - i;
  const u = f * f * (3 - 2 * f);
  return hash(i) * (1 - u) + hash(i + 1) * u;
}

/**
 * Filamentos finos del disco: una tabla de brillo según el radio, irregular y
 * con muchas líneas delgadas, como las bandas de la imagen de referencia.
 */
const STREAKS = (() => {
  const table = new Float32Array(STREAK_SIZE);
  for (let i = 0; i < STREAK_SIZE; i += 1) {
    const r = i * 0.09;
    const raw =
      0.5 * valueNoise(r * 0.9, 1) +
      0.3 * valueNoise(r * 2.3, 2) +
      0.2 * valueNoise(r * 4.2, 3);
    table[i] = Math.pow(Math.min(Math.max(raw, 0), 1), 1.9);
  }
  return table;
})();

/**
 * Pinta el brillo del disco en `image` (RGBA) para el instante `time` (s). El
 * disco gira con la velocidad kepleriana: Ω = r^(-3/2) (a escala visual).
 */
export function paintTrace(
  trace: BlackHoleTrace,
  image: ImageData,
  palette: Uint8ClampedArray,
  time: number,
) {
  const data = image.data;
  data.fill(0);
  const spin = time * 6.5;
  for (const index of trace.lit) {
    let total = 0;
    for (let order = 0; order < ORDERS; order += 1) {
      const base = (trace.intensity[order] as Float32Array)[index] ?? 0;
      if (base === 0) continue;
      const r = (trace.radius[order] as Float32Array)[index] ?? 0;
      const psi = (trace.azimuth[order] as Float32Array)[index] ?? 0;
      const turn = psi - spin * Math.pow(r, -1.5) * 6;
      // Azimut: manchas suaves que giran; radio: filamentos finos y desiguales.
      const patch =
        0.62 +
        0.22 * Math.sin(2 * turn + 0.18 * r) +
        0.18 * Math.sin(3 * turn + 1.1 + 0.3 * r) +
        0.35 * Math.pow(0.5 + 0.5 * Math.sin(turn + 0.5), 4);
      const line = Math.min(
        STREAK_SIZE - 1,
        Math.max(0, Math.floor((r - INNER_RADIUS) / 0.09)),
      );
      const filament =
        0.3 +
        1.5 *
          (STREAKS[line] ?? 0) *
          (0.75 + 0.25 * Math.sin(2 * turn + r * 0.8));
      const texture = patch * filament;
      total += base * texture;
    }
    const value = 1 - Math.exp(-total * 6.4 * (trace.fade[index] ?? 0));
    const level = Math.min(
      255,
      Math.max(0, Math.round(Math.pow(value, 1.25) * 255)),
    );
    const offset = index * 4;
    data[offset] = palette[level * 3] ?? 0;
    data[offset + 1] = palette[level * 3 + 1] ?? 0;
    data[offset + 2] = palette[level * 3 + 2] ?? 0;
    data[offset + 3] = 255;
  }
}
