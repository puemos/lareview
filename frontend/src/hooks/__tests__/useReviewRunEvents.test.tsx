import { act, renderHook, waitFor } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import type { ReactNode } from 'react';
import { useReviewRunEvents } from '../useReviewRunEvents';
import { mockTauri } from '../../test/mocks';
import type { ReviewRunEvent } from '../../types';

const eventMock = vi.hoisted(() => ({ listener: null as ((event: unknown) => void) | null }));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(async (_eventName: string, listener: (event: unknown) => void) => {
    eventMock.listener = listener;
    return vi.fn();
  }),
}));

vi.mock('../useTauri', () => ({
  useTauri: () => mockTauri,
}));

const runEvent = (id: number, runId = 'run-one'): ReviewRunEvent => ({
  id,
  review_id: 'review-one',
  run_id: runId,
  payload: { event: 'Log', data: `Event ${id}` },
  created_at: new Date(id * 1000).toISOString(),
});

describe('useReviewRunEvents', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    eventMock.listener = null;
    mockTauri.getReviewRunEvents.mockResolvedValue([runEvent(1)]);
  });

  it('merges persisted and live activity in order without duplicates or cross-run events', async () => {
    const queryClient = new QueryClient({
      defaultOptions: { queries: { retry: false } },
    });
    const wrapper = ({ children }: { children: ReactNode }) => (
      <QueryClientProvider client={queryClient}>{children}</QueryClientProvider>
    );
    const { result } = renderHook(() => useReviewRunEvents('run-one'), { wrapper });

    await waitFor(() => expect(result.current.data?.map(event => event.id)).toEqual([1]));
    await waitFor(() => expect(eventMock.listener).not.toBeNull());

    act(() => {
      eventMock.listener?.({ payload: runEvent(3) });
      eventMock.listener?.({ payload: runEvent(2) });
      eventMock.listener?.({ payload: runEvent(2) });
      eventMock.listener?.({ payload: runEvent(4, 'run-two') });
    });

    await waitFor(() => {
      expect(result.current.data?.map(event => event.id)).toEqual([1, 2, 3]);
    });
  });
});
