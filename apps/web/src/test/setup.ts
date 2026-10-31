import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach, vi } from 'vitest';

vi.mock('../components/ScanLogTerminal', () => ({
  ScanLogTerminal: () => null,
}));

afterEach(() => {
  cleanup();
});
