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
export const FIELD_WIDTH = 30;
export const FIELD_HEIGHT = 18;
/** Radio de la sombra de un agujero de Schwarzschild: 3√3 M. */
export const SHADOW_RADIUS = 5.196;

const INNER_RADIUS = 6;
const OUTER_RADIUS = 22;
const STEP = 0.035;
const MAX_SWEEP = Math.PI * 3.2;
const ORDERS = 3;
/** Inclinación respecto del eje del disco: 78° (casi de canto, como Gargantúa). */
const INCLINATION = (78 * Math.PI) / 180;

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
    fade[index] = smooth((1 - nx) / 0.45) * smooth((1 - ny) / 0.5);
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
            Math.sqrt(Math.max(1 - 3 / r, 0.001)) /
            Math.max(1 + omega * x * sinTheta, 0.2);
          const edge = Math.min(1, (r - INNER_RADIUS) / 1.2 + 0.15);
          const emissivity = Math.pow(INNER_RADIUS / r, 2.3) * edge;
          const slot = order;
          (trace.intensity[slot] as Float32Array)[index] =
            Math.pow(g, 3.2) * emissivity;
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
    [0, 6, 2, 1],
    [0.22, 96, 20, 4],
    [0.45, 226, 82, 10],
    [0.7, 255, 164, 48],
    [1, 255, 238, 176],
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
  const spin = time * 2.4;
  for (const index of trace.lit) {
    let total = 0;
    for (let order = 0; order < ORDERS; order += 1) {
      const base = (trace.intensity[order] as Float32Array)[index] ?? 0;
      if (base === 0) continue;
      const r = (trace.radius[order] as Float32Array)[index] ?? 0;
      const psi = (trace.azimuth[order] as Float32Array)[index] ?? 0;
      const turn = psi - spin * Math.pow(r, -1.5) * 6;
      // Turbulencia suave: armónicos azimutales que barren en espiral al girar,
      // sin términos de radio de alta frecuencia (que dibujarían anillos finos).
      const texture =
        0.74 +
        0.14 * Math.sin(2 * turn + 0.18 * r) +
        0.12 * Math.sin(3 * turn + 1.1 + 0.3 * r) +
        0.09 * Math.sin(5 * turn + 2.3 + 0.42 * r) +
        0.07 * Math.sin(7 * turn - 0.6 + 0.55 * r) +
        0.05 * Math.sin(r * 0.9);
      total += base * texture;
    }
    const value = 1 - Math.exp(-total * 1.7 * (trace.fade[index] ?? 0));
    const level = Math.min(255, Math.max(0, Math.round(value * 255)));
    const offset = index * 4;
    data[offset] = palette[level * 3] ?? 0;
    data[offset + 1] = palette[level * 3 + 1] ?? 0;
    data[offset + 2] = palette[level * 3 + 2] ?? 0;
    data[offset + 3] = 255;
  }
}
