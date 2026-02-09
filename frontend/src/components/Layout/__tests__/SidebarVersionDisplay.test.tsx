import { describe, it, expect, beforeEach, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { Sidebar } from '../Sidebar';
import { useAppStore } from '../../../store';
import type { UpdateInfo } from '../../../hooks/useUpdateCheck';

const { mockOpenUrl, mockUseTauri } = vi.hoisted(() => {
  const mockOpenUrl = vi.fn().mockResolvedValue(undefined);
  const mockUseTauri = vi.fn(() => ({
    getReviewRuns: vi.fn().mockResolvedValue([]),
    deleteReview: vi.fn().mockResolvedValue(undefined),
    openUrl: mockOpenUrl,
  }));
  return { mockOpenUrl, mockUseTauri };
});

vi.mock('../../../hooks/useTauri', async () => {
  const actual =
    await vi.importActual<typeof import('../../../hooks/useTauri')>('../../../hooks/useTauri');
  return {
    ...actual,
    useTauri: mockUseTauri,
  };
});

vi.mock('../../../hooks/useReviews', () => ({
  useReviews: () => ({
    data: [],
    isLoading: false,
    invalidate: vi.fn(),
  }),
}));

const mockOnUpdateClick = vi.fn();

const renderSidebar = (props?: {
  currentVersion?: string | null;
  updateAvailable?: UpdateInfo | null;
  onUpdateClick?: () => void;
}) => {
  const queryClient = new QueryClient({
    defaultOptions: { queries: { retry: false } },
  });

  return render(
    <QueryClientProvider client={queryClient}>
      <Sidebar
        currentView="generate"
        onViewChange={vi.fn()}
        currentVersion={props?.currentVersion ?? null}
        updateAvailable={props?.updateAvailable ?? null}
        onUpdateClick={props?.onUpdateClick ?? mockOnUpdateClick}
      />
    </QueryClientProvider>
  );
};

describe('Sidebar version display', () => {
  beforeEach(() => {
    useAppStore.getState().reset();
    mockOpenUrl.mockClear();
    mockUseTauri.mockClear();
    mockOnUpdateClick.mockClear();
  });

  it('does not render version footer when currentVersion is null', () => {
    renderSidebar({ currentVersion: null });

    expect(screen.queryByText(/^v\d/)).not.toBeInTheDocument();
  });

  it('renders the current version in the footer', () => {
    renderSidebar({ currentVersion: '0.0.32' });

    expect(screen.getByText('v0.0.32')).toBeInTheDocument();
  });

  it('does not show update link when no update is available', () => {
    renderSidebar({ currentVersion: '0.0.32', updateAvailable: null });

    expect(screen.getByText('v0.0.32')).toBeInTheDocument();
    expect(screen.queryByText('Update')).not.toBeInTheDocument();
  });

  it('shows update link when an update is available', () => {
    renderSidebar({
      currentVersion: '0.0.32',
      updateAvailable: {
        latestVersion: '0.0.33',
        releaseUrl: 'https://github.com/puemos/lareview/releases/tag/v0.0.33',
        releaseName: 'v0.0.33',
        releaseNotes: '',
      },
    });

    expect(screen.getByText('v0.0.32')).toBeInTheDocument();
    expect(screen.getByText('Update')).toBeInTheDocument();
  });

  it('calls onUpdateClick when the update link is clicked', () => {
    renderSidebar({
      currentVersion: '0.0.32',
      updateAvailable: {
        latestVersion: '0.0.33',
        releaseUrl: 'https://github.com/puemos/lareview/releases/tag/v0.0.33',
        releaseName: 'v0.0.33',
        releaseNotes: '',
      },
    });

    fireEvent.click(screen.getByText('Update'));

    expect(mockOnUpdateClick).toHaveBeenCalled();
  });
});
