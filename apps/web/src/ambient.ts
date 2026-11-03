import { createContext } from 'react';

/**
 * Matiz (0-360) del ambiente activo. App lo provee; la terminal y el grafo lo
 * leen para pintarse con el mismo acento que el resto de la interfaz.
 */
export const AmbientHueContext = createContext(24);

/** HSL (h en grados, s y l en %) a RGB de 0 a 255. */
export function hslToRgb(
  hue: number,
  saturation: number,
  lightness: number,
): [number, number, number] {
  const s = saturation / 100;
  const l = lightness / 100;
  const k = (n: number) => (n + hue / 30) % 12;
  const a = s * Math.min(l, 1 - l);
  const channel = (n: number) =>
    Math.round(255 * (l - a * Math.max(-1, Math.min(k(n) - 3, 9 - k(n), 1))));
  return [channel(0), channel(8), channel(4)];
}

/** HSL a color hexadecimal, para las APIs que no aceptan hsl() (xterm, cytoscape). */
export function hslToHex(
  hue: number,
  saturation: number,
  lightness: number,
): string {
  const [r, g, b] = hslToRgb(hue, saturation, lightness);
  return `#${[r, g, b].map((v) => v.toString(16).padStart(2, '0')).join('')}`;
}
