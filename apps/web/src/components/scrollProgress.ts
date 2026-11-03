import { useEffect, type RefObject } from 'react';

/**
 * Escribe en `progressRef` el avance (0 arriba … 1 abajo) del scroll de
 * `.main-content`, el contenedor que desplaza la página. `anchor` es cualquier
 * elemento dentro de él.
 */
export function useScrollProgress(
  anchor: RefObject<HTMLElement | null>,
  progressRef: { current: number },
) {
  useEffect(() => {
    const element = anchor.current;
    if (element === null) return;
    const scroller = element.closest<HTMLElement>('.main-content');
    if (scroller === null) {
      progressRef.current = 1;
      return () => {
        progressRef.current = 0;
      };
    }
    const update = () => {
      const range = scroller.scrollHeight - scroller.clientHeight;
      progressRef.current =
        range > 8 ? Math.min(Math.max(scroller.scrollTop / range, 0), 1) : 0;
    };
    update();
    scroller.addEventListener('scroll', update, { passive: true });
    window.addEventListener('resize', update);
    const layoutObserver =
      typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(update);
    for (const child of Array.from(scroller.children)) {
      layoutObserver?.observe(child);
    }
    return () => {
      layoutObserver?.disconnect();
      scroller.removeEventListener('scroll', update);
      window.removeEventListener('resize', update);
      progressRef.current = 0;
    };
  }, [anchor, progressRef]);
}
