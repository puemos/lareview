import { describe, it, expect, vi, beforeEach } from 'vitest';
import { renderHook, act, waitFor } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { GenerationProvider } from './GenerationContext';
import { useGeneration } from './useGeneration';
import { mockTauri } from '../test/mocks';
import { useAppStore } from '../store';
import type { ProgressEventPayload } from '../hooks/useTauri';

vi.mock('@tauri-apps/api/core', () => {
  return {
    Channel: class {
      onmessage: ((payload: ProgressEventPayload) => void) | null = null;
      send = vi.fn();
    },
    invoke: vi.fn(),
  };
});

vi.mock('../hooks/useTauri', () => ({
  useTauri: () => mockTauri,
}));

const createWrapper = () => {
  const queryClient = new QueryClient({
    defaultOptions: {
      queries: {
        retry: false,
      },
    },
  });
  return ({ children }: { children: React.ReactNode }) => (
    <QueryClientProvider client={queryClient}>
      <GenerationProvider>{children}</GenerationProvider>
    </QueryClientProvider>
  );
};

interface MockChannel {
  onmessage: ((payload: ProgressEventPayload) => void) | null;
  send: (payload: ProgressEventPayload) => void;
}

describe('GenerationContext', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    useAppStore.getState().reset();
  });

  it('starts generation and sets isGenerating to true', async () => {
    const { result } = renderHook(() => useGeneration(), {
      wrapper: createWrapper(),
    });

    // Mock generateReview to be slow

    vi.mocked(mockTauri.generateReview).mockImplementation(() => new Promise(() => {}));

    act(() => {
      result.current.startGeneration({
        diffText: 'test diff',
        agentId: 'test-agent',
        repoId: 'test-repo',
        source: { type: 'diff_paste', diff_hash: 'hash' },
      });
    });

    await waitFor(() => expect(useAppStore.getState().isGenerating).toBe(true));
  });

  it('stops generating when Completed event is received', async () => {
    const { result } = renderHook(() => useGeneration(), {
      wrapper: createWrapper(),
    });

    let channelInstance: MockChannel | null = null;

    vi.mocked(mockTauri.generateReview).mockImplementation(async (...args: unknown[]) => {
      channelInstance = (args[6] as MockChannel | undefined) || null;
      // Keep the promise pending until we send the Completed event
      return new Promise(() => {});
    });

    act(() => {
      result.current.startGeneration({
        diffText: 'test diff',
        agentId: 'test-agent',
        repoId: 'test-repo',
        source: { type: 'diff_paste', diff_hash: 'hash' },
      });
    });

    await waitFor(() => expect(channelInstance).not.toBeNull());

    expect(useAppStore.getState().isGenerating).toBe(true);

    await act(async () => {
      channelInstance?.onmessage?.({ event: 'Completed', data: { task_count: 5 } });
    });

    // This is expected to FAIL before the fix
    expect(useAppStore.getState().isGenerating).toBe(false);
  });

  it('stops generating when Error event is received', async () => {
    const { result } = renderHook(() => useGeneration(), {
      wrapper: createWrapper(),
    });

    let channelInstance: MockChannel | null = null;

    vi.mocked(mockTauri.generateReview).mockImplementation(async (...args: unknown[]) => {
      channelInstance = (args[6] as MockChannel | undefined) || null;
      return new Promise(() => {});
    });

    act(() => {
      result.current.startGeneration({
        diffText: 'test diff',
        agentId: 'test-agent',
        repoId: 'test-repo',
        source: { type: 'diff_paste', diff_hash: 'hash' },
      });
    });

    await waitFor(() => expect(channelInstance).not.toBeNull());

    act(() => {
      channelInstance?.onmessage?.({ event: 'Error', data: { message: 'Failed' } });
    });

    expect(useAppStore.getState().isGenerating).toBe(false);
  });
});
