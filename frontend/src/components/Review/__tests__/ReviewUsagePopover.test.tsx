import { fireEvent, render, screen, cleanup, waitFor } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { UsagePopover } from '../ReviewUsagePopover';

afterEach(cleanup);

describe('UsagePopover', () => {
  it('keeps details hidden until clicked and restores focus after Escape', async () => {
    render(
      <UsagePopover
        agentId="claude"
        usage={{
          model: 'Reported model',
          total_tokens: 128400,
          input_tokens: 16400,
          output_tokens: 8000,
          cached_read_tokens: 104000,
          thought_tokens: 2000,
          context_used: 250000,
          context_size: 200000,
          cost: { amount: 0.84, currency: 'USD' },
        }}
      />
    );
    const trigger = screen.getByRole('button', { name: 'Estimated cost $0.84. View AI usage' });
    expect(trigger).toHaveTextContent('$0.84');
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
    trigger.focus();
    fireEvent.click(trigger);
    expect(screen.getByRole('dialog', { name: 'AI usage' })).toHaveFocus();
    expect(screen.getByText('16,400')).toBeInTheDocument();
    expect(screen.getByText('128,400')).toBeInTheDocument();
    expect(screen.getByRole('meter')).toHaveAttribute('aria-valuenow', '100');
    expect(screen.getByText(/Reported model/)).toBeInTheDocument();
    fireEvent.keyDown(document.activeElement || document.body, { key: 'Escape' });
    await waitFor(() => expect(screen.queryByRole('dialog')).not.toBeInTheDocument());
    await waitFor(() => expect(trigger).toHaveFocus());
  });

  it('keeps unknown usage distinct from free usage and supports closing the popover', () => {
    const { unmount } = render(<UsagePopover agentId="codex" usage={{}} />);
    fireEvent.click(screen.getByRole('button', { name: 'View AI usage' }));
    expect(screen.getByText('No usage reported')).toBeInTheDocument();
    expect(screen.queryByText('$0.00')).not.toBeInTheDocument();
    unmount();
    render(
      <UsagePopover
        agentId="codex"
        usage={{ output_tokens: 0, cost: { amount: 0, currency: 'USD' } }}
        isActive
      />
    );
    fireEvent.click(screen.getByRole('button', { name: 'Estimated cost $0.00. View AI usage' }));
    expect(screen.getByText('Live')).toBeInTheDocument();
    expect(screen.getByText('0')).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Close usage details' }));
    expect(screen.queryByRole('dialog')).not.toBeInTheDocument();
  });

  it('lets users retry a failed history request from the popover', () => {
    const retry = vi.fn();
    render(<UsagePopover agentId="claude" usage={{}} isError onRetry={retry} />);
    fireEvent.click(screen.getByRole('button', { name: 'View AI usage' }));
    fireEvent.click(screen.getByRole('button', { name: 'Try again' }));
    expect(retry).toHaveBeenCalledOnce();
  });
});
