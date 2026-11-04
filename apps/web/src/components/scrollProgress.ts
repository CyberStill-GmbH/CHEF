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
      // En pantallas estrechas se desplaza toda la ventana, no el contenedor.
      const own = scroller.scrollHeight - scroller.clientHeight;
      const root = document.documentElement;
      const range = own > 8 ? own : root.scrollHeight - window.innerHeight;
      const top = own > 8 ? scroller.scrollTop : window.scrollY;
      progressRef.current =
        range > 8 ? Math.min(Math.max(top / range, 0), 1) : 0;
    };
    update();
    scroller.addEventListener('scroll', update, { passive: true });
    window.addEventListener('scroll', update, { passive: true });
    window.addEventListener('resize', update);
    const layoutObserver =
      typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(update);
    for (const child of Array.from(scroller.children)) {
      layoutObserver?.observe(child);
    }
    return () => {
      layoutObserver?.disconnect();
      scroller.removeEventListener('scroll', update);
      window.removeEventListener('scroll', update);
      window.removeEventListener('resize', update);
      progressRef.current = 0;
    };
  }, [anchor, progressRef]);
}
