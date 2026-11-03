import { useEffect, useRef } from 'react';

import { prefersReducedMotion } from '../motion';

interface AboutPageProps {
  onNavigate: (workspace: 'recon' | 'sources') => void;
}

const STEPS = [
  {
    title: 'Importar',
    text: 'Resultados de Nmap, eventos JSONL y respuestas archivadas de Censys y urlscan, siempre de fuentes autorizadas.',
  },
  {
    title: 'Normalizar',
    text: 'Cada activo recibe una identidad determinista y los duplicados se unen de forma conservadora, sin perder de dónde salió cada dato.',
  },
  {
    title: 'Correlacionar',
    text: 'Se proponen relaciones entre activos con reglas que se pueden explicar y con la evidencia que las respalda.',
  },
  {
    title: 'Explorar',
    text: 'El mapa, la procedencia y la exportación versionada permiten revisar cada conclusión hasta su origen.',
  },
] as const;

const LEVELS = [
  {
    name: 'Observación',
    example: 'El puerto 443 de un host respondió.',
    text: 'Es lo que una fuente reportó. Se conserva con su evidencia.',
  },
  {
    name: 'Inferencia',
    example: 'Este dominio parece colgar de aquel otro.',
    text: 'Una relación entre fuentes. Es candidata y se muestra como tal.',
  },
  {
    name: 'Vulnerabilidad confirmada',
    example: 'Validada por una persona, con permiso.',
    text: 'Exige una comprobación aparte. CHEF no la da por hecha.',
  },
] as const;

const CAPABILITIES = [
  {
    title: 'Mapa de relaciones',
    text: 'Los activos conectados se agrupan y los que no tienen relaciones quedan aparte, a la vista.',
  },
  {
    title: 'Procedencia siempre visible',
    text: 'Cada activo y cada relación enlaza con la evidencia que la sostiene.',
  },
  {
    title: 'Importaciones locales',
    text: 'Nmap, eventos JSONL, Censys y urlscan se procesan en tu equipo, con hash y linaje.',
  },
  {
    title: 'Alcance literal',
    text: 'Un archivo importado nunca amplía el alcance autorizado: el scope manda.',
  },
  {
    title: 'Exportación versionada',
    text: 'Snapshot 1.0.0 y un informe de evidencia en HTML para compartir el resultado.',
  },
  {
    title: 'Terminal integrada',
    text: 'Comandos locales para ver el estado, clasificar una URL y generar el informe.',
  },
] as const;

const LAYERS = [
  {
    name: 'Fuentes',
    detail: 'Nmap · JSONL · Censys · urlscan',
  },
  {
    name: 'Núcleo de dominio',
    detail: 'TypeScript aislado · núcleo en Rust',
  },
  {
    name: 'Reglas y evidencia',
    detail: 'Identidad · relaciones explicables',
  },
  {
    name: 'Interfaz web',
    detail: 'React · Vite · TypeScript',
  },
] as const;

const PRINCIPLES = [
  'Se trabaja solo con datos y entornos autorizados.',
  'El alcance lo define la configuración, nunca un archivo importado.',
  'Mismos datos, mismos identificadores: los resultados se pueden reproducir.',
  'Toda conclusión se puede seguir hasta su evidencia de origen.',
] as const;

/**
 * Entrada suave de los bloques que quedan bajo el pliegue. El contenido es
 * visible por defecto: el efecto solo atenúa un poco lo que aún no se ha
 * alcanzado, así que si falla nada queda oculto.
 */
function useReveal() {
  const ref = useRef<HTMLDivElement | null>(null);
  useEffect(() => {
    const root = ref.current;
    if (
      root === null ||
      typeof IntersectionObserver === 'undefined' ||
      prefersReducedMotion()
    )
      return;
    const items = Array.from(
      root.querySelectorAll<HTMLElement>('[data-reveal]'),
    );
    const observer = new IntersectionObserver(
      (entries) => {
        for (const entry of entries) {
          if (!entry.isIntersecting) continue;
          entry.target.classList.remove('is-pending');
          observer.unobserve(entry.target);
        }
      },
      { threshold: 0.12 },
    );
    for (const item of items) {
      if (item.getBoundingClientRect().top > window.innerHeight * 0.9) {
        item.classList.add('is-pending');
      }
      observer.observe(item);
    }
    return () => observer.disconnect();
  }, []);
  return ref;
}

export function AboutPage({ onNavigate }: AboutPageProps) {
  const ref = useReveal();
  return (
    <div className="about-page" ref={ref}>
      <section className="about-hero" aria-labelledby="about-title">
        <img
          className="about-logo"
          src="/chef-logo.png"
          alt=""
          width="76"
          height="76"
        />
        <h1 id="about-title">
          Evidencia clara para entender una{' '}
          <em>superficie de ataque autorizada</em>.
        </h1>
        <p className="about-lead">
          CHEF, un proyecto del Centro Cultural de Ciberseguridad, reduce el
          trabajo manual del reconocimiento: reúne resultados activos y señales
          pasivas, quita duplicados sin perder la procedencia y muestra{' '}
          <strong>por qué</strong> dos observaciones se relacionan.
        </p>
        <div className="about-actions">
          <button
            type="button"
            className="about-button is-primary"
            onClick={() => onNavigate('recon')}
          >
            Ver el mapa
          </button>
          <button
            type="button"
            className="about-button"
            onClick={() => onNavigate('sources')}
          >
            Importar evidencia
          </button>
        </div>
      </section>

      <section
        className="about-section"
        aria-labelledby="about-flow"
        data-reveal
      >
        <h2 id="about-flow">
          De los resultados sueltos a una imagen explicable
        </h2>
        <ol className="about-steps">
          {STEPS.map((step, index) => (
            <li key={step.title} style={{ transitionDelay: `${index * 90}ms` }}>
              <span className="about-step-number">{index + 1}</span>
              <h3>{step.title}</h3>
              <p>{step.text}</p>
            </li>
          ))}
        </ol>
      </section>

      <section
        className="about-section"
        aria-labelledby="about-levels"
        data-reveal
      >
        <h2 id="about-levels">Una observación no es una vulnerabilidad</h2>
        <p className="about-copy">
          CHEF mantiene separados tres niveles de certeza para que nadie
          confunda un indicio con un hallazgo.
        </p>
        <ul className="about-levels">
          {LEVELS.map((level, index) => (
            <li key={level.name} data-level={index + 1}>
              <h3>{level.name}</h3>
              <p className="about-example">{level.example}</p>
              <p>{level.text}</p>
            </li>
          ))}
        </ul>
      </section>

      <section
        className="about-section"
        aria-labelledby="about-features"
        data-reveal
      >
        <h2 id="about-features">Capacidades</h2>
        <ul className="about-grid">
          {CAPABILITIES.map((item) => (
            <li key={item.title}>
              <h3>{item.title}</h3>
              <p>{item.text}</p>
            </li>
          ))}
        </ul>
      </section>

      <section
        className="about-section"
        aria-labelledby="about-architecture"
        data-reveal
      >
        <h2 id="about-architecture">Arquitectura</h2>
        <p className="about-copy">
          El dominio no depende de la red, del sistema de archivos ni de ningún
          framework: las fuentes entran por adaptadores y la interfaz solo
          consume el resultado.
        </p>
        <ol className="about-layers">
          {LAYERS.map((layer) => (
            <li key={layer.name}>
              <strong>{layer.name}</strong>
              <span>{layer.detail}</span>
            </li>
          ))}
        </ol>
        <p className="about-note">
          Además existe un adaptador para Burp Suite, en fase de prototipo, que
          normaliza solicitudes HTTP seleccionadas.
        </p>
      </section>

      <section
        className="about-section"
        aria-labelledby="about-principles"
        data-reveal
      >
        <h2 id="about-principles">Principios</h2>
        <ul className="about-principles">
          {PRINCIPLES.map((text) => (
            <li key={text}>{text}</li>
          ))}
        </ul>
      </section>

      <section
        className="about-section"
        aria-labelledby="about-status"
        data-reveal
      >
        <h2 id="about-status">Estado del proyecto</h2>
        <div className="about-status">
          <div>
            <h3>Hoy</h3>
            <p>
              Un explorador web que muestra un análisis de ejemplo: mapa de
              relaciones, evidencia, importaciones y exportación de informe.
            </p>
          </div>
          <div>
            <h3>Siguiente</h3>
            <p>
              Conectar la interfaz a la API en Rust, sumar proyectos por usuario
              y completar las fuentes pasivas autorizadas.
            </p>
          </div>
        </div>
      </section>

      <section
        className="about-section about-credits"
        aria-labelledby="about-credits"
        data-reveal
      >
        <h2 id="about-credits">Licencia y atribución</h2>
        <p className="about-copy">
          CHEF se distribuye bajo AGPL-3.0 y adapta capacidades de BBOT, de
          Black Lantern Security. No está afiliado ni aprobado por PortSwigger
          ni por Nmap.
        </p>
        <p className="about-links">
          <a href="/LICENSE" target="_blank" rel="noreferrer">
            Texto de la licencia
          </a>
          <a href="/THIRD_PARTY_NOTICES.md">Avisos de terceros</a>
        </p>
      </section>
    </div>
  );
}
