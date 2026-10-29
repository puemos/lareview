import { render, screen, fireEvent } from '@testing-library/react';
import { expect, it, vi } from 'vitest';
import { ReviewGenerationActivity } from './ReviewGenerationActivity';
import type { ReviewRun } from '../../types';
const mocks = vi.hoisted(() => ({ stop: vi.fn() }));
vi.mock('../../contexts/useGeneration', () => ({
  useGeneration: () => ({ stopGeneration: mocks.stop }),
}));
vi.mock('../../hooks/useReviewRunEvents', () => ({
  useReviewRunEvents: () => ({ data: [], refetch: vi.fn(), isLoading: false, isError: false }),
}));
vi.mock('./ReviewUsagePopover', () => ({ UsagePopover: () => null }));
it('releases Stop and shows an inline error after a failed cancellation', async () => {
  mocks.stop.mockRejectedValueOnce(new Error('Cancellation unavailable'));
  render(
    <ReviewGenerationActivity
      review={null}
      run={{ id: 'run', status: 'running', agent_id: 'copilot' } as ReviewRun}
    />
  );
  fireEvent.click(screen.getByRole('button', { name: 'Stop' }));
  await screen.findByText('Could not stop review');
  expect(screen.getByRole('button', { name: 'Stop' })).toBeEnabled();
});
