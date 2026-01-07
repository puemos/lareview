import '@testing-library/jest-dom';

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
