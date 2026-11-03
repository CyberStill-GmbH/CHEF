import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';

import { App } from './App.tsx';
import './styles.css';
import './styles/workbench.css';
import './styles/terminal-first.css';

const rootElement = document.getElementById('root');

if (rootElement === null) {
  throw new Error('CHEF web root element was not found.');
}

createRoot(rootElement).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
