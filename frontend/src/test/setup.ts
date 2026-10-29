import { expect } from 'vitest';
import * as matchers from '@testing-library/jest-dom/matchers';
import type { TestingLibraryMatchers } from '@testing-library/jest-dom/matchers';

// jest-dom's bundled Vitest types still target the pre-v5 Assertion interface.
declare module 'vitest' {
  // eslint-disable-next-line @typescript-eslint/no-empty-object-type -- Vitest extends matchers through declaration merging.
  interface Matchers<
    R extends void | Promise<void> = void | Promise<void>,
    T = unknown,
  > extends TestingLibraryMatchers<T, R> {}
}

expect.extend(matchers);

// Mock localStorage for zustand persist middleware
const localStorageMock = {
  getItem: () => null,
  setItem: () => {},
  removeItem: () => {},
  clear: () => {},
};
Object.defineProperty(window, 'localStorage', {
  value: localStorageMock,
  writable: true,
});
