import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { AppearanceSettings } from './AppearanceSettings';
import { APPEARANCE_DEFAULTS, syncAppearance, useAppearanceStore } from '../../store/appearance';

describe('Appearance settings', () => {
  let stored: Map<string, string>;
  let stopSync: () => void;

  beforeEach(() => {
    stored = new Map();
    vi.spyOn(window.localStorage, 'getItem').mockImplementation(key => stored.get(key) ?? null);
    vi.spyOn(window.localStorage, 'setItem').mockImplementation((key, value) => {
      stored.set(key, value);
    });
    useAppearanceStore.getState().resetAppearance();
    stopSync = syncAppearance();
  });

  afterEach(() => {
    cleanup();
    stopSync();
    useAppearanceStore.getState().resetAppearance();
    document.documentElement.style.removeProperty('font-size');
    vi.restoreAllMocks();
  });

  it('previews interface size without applying or persisting it until saved', () => {
    render(<AppearanceSettings />);
    const reset = screen.getByRole('button', { name: 'Restore default sizes' });
    expect(reset).toBeDisabled();

    fireEvent.change(screen.getByRole('slider', { name: 'Interface text' }), {
      target: { value: '150' },
    });
    expect(document.documentElement.style.fontSize).toBe('100%');
    expect(useAppearanceStore.getState().interfaceScale).toBe(100);
    expect(JSON.parse(stored.get('lareview-appearance')!).state.interfaceScale).toBe(100);
    expect(screen.getByRole('heading', { name: 'Review with confidence' }).style.fontSize).toBe(
      '1.3125rem'
    );
    expect(
      screen.getByText('function greet(name: string)', { exact: false }).parentElement
    ).toHaveStyle({ fontSize: '12px' });

    fireEvent.change(screen.getByRole('slider', { name: 'Code text' }), {
      target: { value: '20' },
    });
    expect(document.documentElement.style.fontSize).toBe('100%');
    expect(
      screen.getByText('function greet(name: string)', { exact: false }).parentElement
    ).toHaveStyle({ fontSize: '20px' });

    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(document.documentElement.style.fontSize).toBe('150%');
    expect(JSON.parse(stored.get('lareview-appearance')!).state.interfaceScale).toBe(150);
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
    expect(screen.getByRole('heading', { name: 'Review with confidence' }).style.fontSize).toBe(
      '0.875rem'
    );

    fireEvent.click(reset);
    expect(document.documentElement.style.fontSize).toBe('150%');
    fireEvent.click(screen.getByRole('button', { name: 'Save changes' }));
    expect(document.documentElement.style.fontSize).toBe('100%');
    expect(useAppearanceStore.getState()).toMatchObject(APPEARANCE_DEFAULTS);
    expect(reset).toBeDisabled();
  });

  it('discards an unsaved interface draft when leaving settings', () => {
    const { unmount } = render(<AppearanceSettings />);
    fireEvent.change(screen.getByRole('slider', { name: 'Interface text' }), {
      target: { value: '200' },
    });
    unmount();
    render(<AppearanceSettings />);
    expect(screen.getByRole('slider', { name: 'Interface text' })).toHaveValue('100');
    expect(document.documentElement.style.fontSize).toBe('100%');
    expect(screen.getByRole('button', { name: 'Save changes' })).toBeDisabled();
  });

  it('restores both sizes from persisted preferences before applying interface sizing', async () => {
    useAppearanceStore.getState().setInterfaceScale(180);
    useAppearanceStore.getState().setCodeFontSize(22);
    const saved = stored.get('lareview-appearance')!;
    expect(JSON.parse(saved).state).toEqual({ interfaceScale: 180, codeFontSize: 22 });

    useAppearanceStore.getState().resetAppearance();
    stored.set('lareview-appearance', saved);
    await useAppearanceStore.persist.rehydrate();

    expect(useAppearanceStore.getState()).toMatchObject({ interfaceScale: 180, codeFontSize: 22 });
    expect(document.documentElement.style.fontSize).toBe('180%');
  });

  it('recovers safely from invalid or out-of-range saved preferences', async () => {
    stored.set(
      'lareview-appearance',
      JSON.stringify({
        state: { interfaceScale: 'huge', codeFontSize: 900 },
        version: 0,
      })
    );
    await useAppearanceStore.persist.rehydrate();
    expect(useAppearanceStore.getState()).toMatchObject({ interfaceScale: 100, codeFontSize: 24 });

    useAppearanceStore.getState().setInterfaceScale(Infinity);
    useAppearanceStore.getState().setCodeFontSize(NaN);
    expect(useAppearanceStore.getState()).toMatchObject(APPEARANCE_DEFAULTS);
  });
});
