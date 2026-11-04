/**
 * Las animaciones de CHEF son decorativas. Se desactivan cuando la persona pide
 * "reducir movimiento" y también donde no existe matchMedia (p. ej. jsdom), de
 * modo que el contenido siempre aparece completo y estático en esos casos.
 */
export function prefersReducedMotion(): boolean {
  return (
    typeof window === 'undefined' ||
    typeof window.matchMedia !== 'function' ||
    window.matchMedia('(prefers-reduced-motion: reduce)').matches
  );
}
