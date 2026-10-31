import { create } from 'zustand';
import { persist } from 'zustand/middleware';

export const APPEARANCE_DEFAULTS = { interfaceScale: 100, codeFontSize: 12 };
export const INTERFACE_SCALE = { min: 100, max: 200, step: 10 };
export const CODE_FONT_SIZE = { min: 10, max: 24, step: 1 };

interface AppearanceStore {
  interfaceScale: number;
  codeFontSize: number;
  setInterfaceScale: (value: number) => void;
  setCodeFontSize: (value: number) => void;
  resetAppearance: () => void;
}

function normalize(value: unknown, fallback: number, min: number, max: number) {
  return typeof value === 'number' && Number.isFinite(value)
    ? Math.round(Math.min(max, Math.max(min, value)))
    : fallback;
}

export const useAppearanceStore = create<AppearanceStore>()(
  persist(
    set => ({
      ...APPEARANCE_DEFAULTS,
      setInterfaceScale: value =>
        set({
          interfaceScale: normalize(
            value,
            APPEARANCE_DEFAULTS.interfaceScale,
            INTERFACE_SCALE.min,
            INTERFACE_SCALE.max
          ),
        }),
      setCodeFontSize: value =>
        set({
          codeFontSize: normalize(
            value,
            APPEARANCE_DEFAULTS.codeFontSize,
            CODE_FONT_SIZE.min,
            CODE_FONT_SIZE.max
          ),
        }),
      resetAppearance: () => set(APPEARANCE_DEFAULTS),
    }),
    {
      name: 'lareview-appearance',
      partialize: ({ interfaceScale, codeFontSize }) => ({ interfaceScale, codeFontSize }),
      merge: (persisted, current) => {
        const saved = persisted as Partial<AppearanceStore> | null;
        return {
          ...current,
          interfaceScale: normalize(
            saved?.interfaceScale,
            APPEARANCE_DEFAULTS.interfaceScale,
            INTERFACE_SCALE.min,
            INTERFACE_SCALE.max
          ),
          codeFontSize: normalize(
            saved?.codeFontSize,
            APPEARANCE_DEFAULTS.codeFontSize,
            CODE_FONT_SIZE.min,
            CODE_FONT_SIZE.max
          ),
        };
      },
    }
  )
);

// Apply persisted sizing before React paints, including content rendered in portals.
export function syncAppearance() {
  const apply = () => {
    document.documentElement.style.fontSize = `${useAppearanceStore.getState().interfaceScale}%`;
  };
  apply();
  return useAppearanceStore.subscribe(apply);
}
