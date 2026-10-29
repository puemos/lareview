import { describe, expect, it, vi } from 'vitest';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { LocalGitInput } from '../LocalGitInput';
import type { ReviewSource } from '../../../types';

const source: ReviewSource = {
  type: 'local_git',
  repo_id: 'repo',
  base_ref: 'release/v1',
  head_ref: 'feature/greeting',
  base_sha: 'a'.repeat(40),
  head_sha: 'b'.repeat(40),
};

const props = {
  repoName: 'Internal repository',
  source,
  compact: true,
  isLoading: false,
  disabled: false,
  error: null,
  onCompare: vi.fn(),
  onLinkRepo: vi.fn(),
};

describe('LocalGitInput', () => {
  it('opens the loaded refs for editing and restores them when cancelled', () => {
    const onCancelEdit = vi.fn();
    const { rerender } = render(<LocalGitInput {...props} onCancelEdit={onCancelEdit} />);
    expect(screen.queryByRole('textbox')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit comparison' }));
    fireEvent.change(screen.getByLabelText('Base ref'), { target: { value: 'missing' } });
    rerender(<LocalGitInput {...props} error="Unknown ref" onCancelEdit={onCancelEdit} />);
    expect(screen.getByLabelText('Base ref')).toHaveAttribute('aria-invalid', 'true');
    expect(screen.getByRole('alert')).toHaveTextContent('Unknown ref');
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    expect(onCancelEdit).toHaveBeenCalledOnce();
    expect(screen.queryByRole('alert')).not.toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Edit comparison' }));
    expect(screen.getByLabelText('Base ref')).toHaveValue('release/v1');
    expect(screen.getByLabelText('Head ref')).toHaveValue('feature/greeting');
  });

  it('keeps failed edits open and collapses only after a successful comparison', async () => {
    const onCompare = vi.fn().mockResolvedValueOnce(false).mockResolvedValueOnce(true);
    render(<LocalGitInput {...props} onCompare={onCompare} />);
    fireEvent.click(screen.getByRole('button', { name: 'Edit comparison' }));
    fireEvent.change(screen.getByLabelText('Base ref'), { target: { value: ' main ' } });
    fireEvent.click(screen.getByRole('button', { name: 'Compare changes' }));
    await waitFor(() => expect(onCompare).toHaveBeenCalledWith('main', 'feature/greeting'));
    expect(screen.getByRole('textbox', { name: 'Base ref' })).toBeInTheDocument();
    fireEvent.click(screen.getByRole('button', { name: 'Compare changes' }));
    await waitFor(() => expect(screen.queryByRole('textbox')).not.toBeInTheDocument());
  });
});
