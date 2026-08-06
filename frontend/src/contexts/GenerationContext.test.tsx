import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GenerationProvider } from './GenerationContext';
import { useGeneration } from './useGeneration';
import { mockTauri } from '../test/mocks';
import { useAppStore } from '../store';

const eventMock = vi.hoisted(() => ({ listener: null as ((event: unknown) => void) | null }));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_eventName: string, listener: (event: unknown) => void) => {
    eventMock.listener = listener;
    return vi.fn();
  }),
}));

vi.mock('../hooks/useTauri', () => ({
  useTauri: () => mockTauri,
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: { retry: false },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <GenerationProvider>{children}</GenerationProvider>
    </QueryClientProvider>
  );
};

describe('GenerationContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.getState().reset();
    mockTauri.generateReview.mockResolvedValue({
      review_id: 'review-new',
      run_id: 'run-new',
      status: 'queued',
    });
  });

  it('returns the new review handle as soon as generation is queued', async () => {
    const { result } = renderHook(() => useGeneration(), { wrapper: createWrapper() });

    let createdReview: Awaited<ReturnType<typeof result.current.startGeneration>> = null;
    await act(async () => {
      createdReview = await result.current.startGeneration({
        diffText: 'diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts',
        agentId: 'test-agent',
        repoId: 'test-repo',
        source: { type: 'diff_paste', diff_hash: 'hash' },
      });
    });

    expect(createdReview).toEqual({
      review_id: 'review-new',
      run_id: 'run-new',
      status: 'queued',
    });
    expect(mockTauri.generateReview).toHaveBeenCalledWith(
      'diff --git a/a.ts b/a.ts\n--- a/a.ts\n+++ b/a.ts',
      'test-agent',
      'test-repo',
      { type: 'diff_paste', diff_hash: 'hash' },
      false,
      undefined
    );
  });

  it('stops the requested run without relying on a global active run', async () => {
    const { result } = renderHook(() => useGeneration(), { wrapper: createWrapper() });

    await act(async () => {
      await result.current.stopGeneration('run-two');
    });

    expect(mockTauri.stop_generation).toHaveBeenCalledWith('run-two');
  });
});
