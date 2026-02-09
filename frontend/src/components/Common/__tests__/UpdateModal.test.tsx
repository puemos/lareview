import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { UpdateModal } from '../UpdateModal';
import type { UpdateInfo } from '../../../hooks/useUpdateCheck';

const { mockCopyToClipboard, mockOpenUrl } = vi.hoisted(() => {
  const mockCopyToClipboard = vi.fn().mockResolvedValue(undefined);
  const mockOpenUrl = vi.fn().mockResolvedValue(undefined);
  return { mockCopyToClipboard, mockOpenUrl };
});

vi.mock('../../../hooks/useTauri', () => ({
  useTauri: () => ({
    copyToClipboard: mockCopyToClipboard,
    openUrl: mockOpenUrl,
  }),
}));

vi.mock('../../ui/MarkdownRenderer', () => ({
  MarkdownRenderer: ({ children }: { children: string }) => (
    <div data-testid="markdown-renderer">{children}</div>
  ),
}));

const defaultUpdateInfo: UpdateInfo = {
  latestVersion: '0.0.33',
  releaseUrl: 'https://github.com/puemos/lareview/releases/tag/v0.0.33',
  releaseName: 'v0.0.33',
  releaseNotes: '## Bug Fixes\n- Fixed a thing',
};

const renderModal = (props?: Partial<Parameters<typeof UpdateModal>[0]>) =>
  render(
    <UpdateModal
      isOpen={true}
      onClose={vi.fn()}
      currentVersion="0.0.32"
      updateInfo={defaultUpdateInfo}
      {...props}
    />
  );

describe('UpdateModal', () => {
  beforeEach(() => {
    mockCopyToClipboard.mockClear();
    mockOpenUrl.mockClear();
  });

  it('does not render when isOpen is false', () => {
    renderModal({ isOpen: false });

    expect(screen.queryByText('Update Available')).not.toBeInTheDocument();
  });

  it('renders the version transition pill', () => {
    renderModal();

    expect(screen.getByText('v0.0.32 → v0.0.33')).toBeInTheDocument();
  });

  it('renders release notes via MarkdownRenderer', () => {
    renderModal();

    const md = screen.getByTestId('markdown-renderer');
    expect(md).toHaveTextContent('## Bug Fixes');
  });

  it('shows fallback when release notes are empty', () => {
    renderModal({
      updateInfo: { ...defaultUpdateInfo, releaseNotes: '' },
    });

    expect(screen.getByText('No release notes available for this version.')).toBeInTheDocument();
  });

  it('shows the brew upgrade command', () => {
    renderModal();

    expect(screen.getByText('brew upgrade --cask lareview')).toBeInTheDocument();
  });

  it('calls copyToClipboard when copy button is clicked', () => {
    renderModal();

    fireEvent.click(screen.getByLabelText('Copy command'));

    expect(mockCopyToClipboard).toHaveBeenCalledWith('brew upgrade --cask lareview');
  });

  it('calls copyToClipboard when footer copy button is clicked', () => {
    renderModal();

    fireEvent.click(screen.getByText('Copy Update Command'));

    expect(mockCopyToClipboard).toHaveBeenCalledWith('brew upgrade --cask lareview');
  });

  it('calls openUrl when View on GitHub is clicked', () => {
    renderModal();

    fireEvent.click(screen.getByText('View on GitHub'));

    expect(mockOpenUrl).toHaveBeenCalledWith(
      'https://github.com/puemos/lareview/releases/tag/v0.0.33'
    );
  });

  it('calls onClose when close button is clicked', () => {
    const onClose = vi.fn();
    renderModal({ onClose });

    fireEvent.click(screen.getByLabelText('Close'));

    expect(onClose).toHaveBeenCalled();
  });
});
