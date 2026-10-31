import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

vi.mock('../components/ScanLogTerminal', () => ({
  ScanLogTerminal: () => null,
}));

// App state tests run in jsdom, which has no canvas implementation; these tests
// do not exercise the Cytoscape renderer.
vi.mock('../components/ReconGraphPanel', () => ({
  ReconGraphPanel: () => null,
}));

afterEach(() => {
  cleanup();
});
