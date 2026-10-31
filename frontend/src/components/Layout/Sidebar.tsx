import React, { useCallback, useState } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../store';
import { ConfirmationModal } from '../Common/ConfirmationModal';
import { Brain } from '@phosphor-icons/react';
import { ICONS } from '../../constants/icons';
import type { ViewType } from '../../types';
import { useReviews } from '../../hooks/useReviews';
import { useTauri } from '../../hooks/useTauri';
import { queryKeys } from '../../lib/query-keys';
import type { UpdateInfo } from '../../hooks/useUpdateCheck';
import { WindowDragRegion } from './WindowDragRegion';

interface SidebarProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
  currentVersion: string | null;
  updateAvailable: UpdateInfo | null;
  onUpdateClick?: () => void;
}

export const Sidebar: React.FC<SidebarProps> = ({
  currentView,
  onViewChange,
  currentVersion,
  updateAvailable,
  onUpdateClick,
}) => {
  const queryClient = useQueryClient();
  const { setReviewId, reviewId } = useAppStore();
  const { data: reviews = [], isLoading, invalidate } = useReviews();
  const { deleteReview, getReviewRuns } = useTauri();
  const [error, setError] = useState<string | null>(null);
  const [reviewToDelete, setReviewToDelete] = useState<string | null>(null);

  const prefetchReview = useCallback(
    (reviewId: string) => {
      queryClient.prefetchQuery({
        queryKey: queryKeys.reviewRuns(reviewId),
        queryFn: () => getReviewRuns(reviewId),
        staleTime: 1000 * 30,
      });
    },
    [queryClient, getReviewRuns]
  );

  const handleReviewClick = (id: string) => {
    setError(null);
    setReviewId(id);
    onViewChange('review');
  };

  const handleDeleteReview = async (e: React.MouseEvent, id: string) => {
    e.stopPropagation();
    setError(null);
    setReviewToDelete(id);
  };

  const confirmDeleteReview = async () => {
    if (!reviewToDelete) return;
    try {
      await deleteReview(reviewToDelete);
      invalidate();
      if (reviewId === reviewToDelete) {
        setReviewId(null);
        onViewChange('generate');
      }
      setReviewToDelete(null);
      toast('Review Deleted', {
        description: 'The review and all associated data were removed.',
      });
    } catch (err) {
      console.error('Failed to delete review:', err);
      toast('Failed to delete review', {
        description: err instanceof Error ? err.message : String(err),
      });
      setReviewToDelete(null);
    }
  };

  return (
    <aside
      className="border-border flex max-h-screen w-56 shrink-0 flex-col gap-0.5 overflow-x-hidden overflow-y-auto border-r bg-gray-950/50 max-[1000px]:w-48"
      role="navigation"
      aria-label="Main sidebar"
    >
      <WindowDragRegion className="h-10 w-full shrink-0" />
      {error && (
        <div className="bg-status-ignored/10 border-status-ignored/20 text-status-ignored mx-2 rounded-md border px-3 py-2 text-[0.625rem]">
          {error}
        </div>
      )}
      <SidebarItem
        icon={<ICONS.VIEW_GENERATE size={16} />}
        label="Generate Review"
        isActive={currentView === 'generate'}
        onClick={() => onViewChange('generate')}
        ariaLabel="Navigate to Generate Review"
      />

      <div className="flex flex-col gap-0.5">
        <SidebarItem
          icon={<ICONS.VIEW_REVIEW size={16} />}
          label="Reviews"
          isActive={currentView === 'review' && !reviewId}
          onClick={() => {
            onViewChange('review');
            if (reviews.length > 0 && !reviewId) {
              setReviewId(reviews[0].id);
            }
          }}
          ariaLabel="Navigate to Reviews"
        />
        {reviews.length > 0 && !isLoading && (
          <div className="relative my-1 flex flex-col gap-0.5 pl-4">
            <div className="bg-border/50 absolute top-0 bottom-0 left-6 w-px" />
            <div className="custom-scrollbar flex max-h-[300px] flex-col gap-0.5 overflow-y-auto pr-1">
              {reviews.map(review => (
                <div
                  key={review.id}
                  className={`group relative ml-2 flex items-center rounded-md text-left text-xs transition-colors ${
                    reviewId === review.id && currentView === 'review'
                      ? 'text-text-primary bg-white/[0.065]'
                      : 'text-gray-500 hover:bg-white/5 hover:text-gray-300'
                  }`}
                  onMouseEnter={() => prefetchReview(review.id)}
                >
                  <button
                    type="button"
                    className="flex min-w-0 flex-1 items-center gap-2 rounded-md px-3 py-2 text-left"
                    onClick={() => handleReviewClick(review.id)}
                    aria-label={`Review: ${review.title}`}
                    aria-current={
                      reviewId === review.id && currentView === 'review' ? 'page' : undefined
                    }
                    title={review.title}
                  >
                    <span
                      className="flex h-4 w-4 shrink-0 items-center justify-center"
                      aria-hidden="true"
                    >
                      {review.active_run_status === 'running' ||
                      review.active_run_status === 'queued' ? (
                        <ICONS.ACTION_LOADING
                          size={10}
                          className="animate-spin text-blue-400"
                          aria-hidden="true"
                        />
                      ) : (
                        <span
                          className={`h-1.5 w-1.5 rounded-full ${review.status === 'done' ? 'bg-green-500/50' : 'bg-blue-500/50'}`}
                        />
                      )}
                    </span>
                    <span className="flex-1 truncate">{review.title}</span>
                  </button>
                  <button
                    onClick={e => handleDeleteReview(e, review.id)}
                    className="text-text-disabled z-10 mr-1 shrink-0 rounded-md p-1 opacity-0 transition-opacity group-focus-within:opacity-100 group-hover:opacity-100 hover:bg-red-400/10 hover:text-red-400"
                    title="Delete Review"
                    type="button"
                    aria-label={`Delete review: ${review.title}`}
                  >
                    <ICONS.ACTION_DELETE size={14} aria-hidden="true" />
                  </button>
                </div>
              ))}
            </div>
          </div>
        )}
      </div>

      <SidebarItem
        icon={<ICONS.VIEW_INBOX size={16} />}
        label="Inbox"
        isActive={currentView === 'inbox'}
        onClick={() => onViewChange('inbox')}
        ariaLabel="Navigate to Inbox"
      />

      <p className="text-text-disabled mt-5 mb-2 px-5 text-[0.625rem] font-medium tracking-[0.08em] uppercase">
        Workspace
      </p>

      <SidebarItem
        icon={<ICONS.VIEW_REPOS size={16} />}
        label="Repositories"
        isActive={currentView === 'repos'}
        onClick={() => onViewChange('repos')}
        ariaLabel="Navigate to Repositories"
      />
      <SidebarItem
        icon={<ICONS.VIEW_RULES size={16} />}
        label="Rules"
        isActive={currentView === 'rules'}
        onClick={() => onViewChange('rules')}
        ariaLabel="Navigate to Rules"
      />
      <SidebarItem
        icon={<Brain size={16} weight="fill" />}
        label="Learning"
        isActive={currentView === 'learning'}
        onClick={() => onViewChange('learning')}
        ariaLabel="Navigate to Learning"
      />
      <div className="flex-1" />
      <SidebarItem
        icon={<ICONS.VIEW_SETTINGS size={16} />}
        label="Settings"
        isActive={currentView === 'settings'}
        onClick={() => onViewChange('settings')}
        ariaLabel="Navigate to Settings"
      />

      <div className="h-2" />

      <div className="border-border/40 flex h-10 shrink-0 items-center justify-between border-t px-5">
        <span className="text-text-tertiary text-[0.6875rem] font-medium">LaReview</span>
        {currentVersion ? (
          <span className="text-text-disabled text-[0.625rem]">v{currentVersion}</span>
        ) : (
          <div className="bg-bg-tertiary h-3 w-12 animate-pulse rounded" />
        )}
        {updateAvailable && (
          <button
            onClick={() => onUpdateClick?.()}
            className="text-brand/70 hover:text-brand flex items-center gap-1 text-[0.625rem] transition-colors"
          >
            <ICONS.ARROW_UP size={10} />
            <span>Update</span>
          </button>
        )}
      </div>

      <ConfirmationModal
        isOpen={!!reviewToDelete}
        onClose={() => setReviewToDelete(null)}
        onConfirm={confirmDeleteReview}
        title="Delete Review"
        message="Are you sure you want to delete this review? All associated feedback and data will be permanently removed."
        confirmLabel="Delete"
        confirmVariant="danger"
      />
    </aside>
  );
};

interface SidebarItemProps {
  icon: React.ReactNode;
  label: string;
  isActive: boolean;
  onClick: () => void;
  ariaLabel?: string;
  suffix?: React.ReactNode;
}

const SidebarItem: React.FC<SidebarItemProps> = ({
  icon,
  label,
  isActive,
  onClick,
  ariaLabel,
  suffix,
}) => (
  <button
    onClick={onClick}
    className={`group mx-2 flex min-h-9 items-center gap-2.5 rounded-md px-3 py-2 text-[0.8125rem] transition-colors select-none ${
      isActive
        ? 'text-text-primary bg-white/[0.065]'
        : 'text-text-tertiary hover:text-text-primary hover:bg-white/[0.035]'
    }`}
    aria-label={ariaLabel || label}
    aria-current={isActive ? 'page' : undefined}
  >
    <div
      className={`transition-colors ${isActive ? 'text-brand' : 'text-text-disabled group-hover:text-text-secondary'}`}
      aria-hidden="true"
    >
      {icon}
    </div>
    <span className="flex-1 text-left font-medium">{label}</span>
    {suffix}
  </button>
);
