import React, { useCallback, useState } from 'react';
import { toast } from 'sonner';
import { useQueryClient } from '@tanstack/react-query';
import { useAppStore } from '../../store';
import { ConfirmationModal } from '../Common/ConfirmationModal';
import { ICONS } from '../../constants/icons';
import type { ViewType } from '../../types';
import { useReviews } from '../../hooks/useReviews';
import { useTauri } from '../../hooks/useTauri';

interface SidebarProps {
  currentView: ViewType;
  onViewChange: (view: ViewType) => void;
}

export const Sidebar: React.FC<SidebarProps> = ({ currentView, onViewChange }) => {
  const queryClient = useQueryClient();
  const { setReviewId, reviewId } = useAppStore();
  const { data: reviews = [], isLoading, invalidate } = useReviews();
  const { deleteReview } = useTauri();
  const [error, setError] = useState<string | null>(null);
  const [reviewToDelete, setReviewToDelete] = useState<string | null>(null);

  const prefetchReview = useCallback(
    (reviewId: string) => {
      queryClient.prefetchQuery({
        queryKey: ['reviewRuns', reviewId],
        queryFn: async () => {
          const { getReviewRuns } = await import('../../hooks/useTauri').then(m => m.useTauri());
          return getReviewRuns(reviewId);
        },
        staleTime: 1000 * 30,
      });
    },
    [queryClient]
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
      className="border-border flex max-h-screen w-64 flex-col gap-0.5 overflow-hidden border-r bg-gray-950/50 py-2"
      role="navigation"
      aria-label="Main sidebar"
    >
      {error && (
        <div className="bg-status-ignored/10 border-status-ignored/20 text-status-ignored mx-2 rounded-md border px-3 py-2 text-[10px]">
          {error}
        </div>
      )}
      <div className="mb-2 shrink-0 px-3 text-[10px] font-bold tracking-wider text-gray-500 uppercase">
        Workspace
      </div>
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
                  role="button"
                  tabIndex={0}
                  className={`group relative ml-2 flex cursor-pointer items-center gap-2 rounded-md px-3 py-1.5 text-left text-xs transition-all ${
                    reviewId === review.id && currentView === 'review'
                      ? 'bg-blue-500/10 text-blue-400'
                      : 'text-gray-500 hover:bg-white/5 hover:text-gray-300'
                  }`}
                  onClick={() => handleReviewClick(review.id)}
                  onKeyDown={e => e.key === 'Enter' && handleReviewClick(review.id)}
                  onMouseEnter={() => prefetchReview(review.id)}
                  aria-label={`Review: ${review.title}`}
                  aria-current={
                    reviewId === review.id && currentView === 'review' ? 'page' : undefined
                  }
                >
                  <span
                    className="flex h-4 w-4 shrink-0 items-center justify-center"
                    aria-hidden="true"
                  >
                    {review.status === 'in_progress' ? (
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
                  <button
                    onClick={e => handleDeleteReview(e, review.id)}
                    className="z-10 shrink-0 rounded-md p-1 text-gray-500/50 opacity-0 transition-all group-hover:opacity-100 hover:bg-red-400/10 hover:text-red-400"
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
        icon={<ICONS.VIEW_REPOS size={16} />}
        label="Repositories"
        isActive={currentView === 'repos'}
        onClick={() => onViewChange('repos')}
        ariaLabel="Navigate to Repositories"
      />
      <div className="flex-1" />
      <SidebarItem
        icon={<ICONS.VIEW_SETTINGS size={16} />}
        label="Settings"
        isActive={currentView === 'settings'}
        onClick={() => onViewChange('settings')}
        ariaLabel="Navigate to Settings"
      />

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
    className={`group mx-2 flex items-center gap-2.5 rounded-md px-3 py-1.5 text-sm transition-all select-none ${
      isActive
        ? 'bg-blue-500/10 text-blue-400'
        : 'text-gray-400 hover:bg-gray-800/50 hover:text-gray-200'
    }`}
    aria-label={ariaLabel || label}
    aria-current={isActive ? 'page' : undefined}
  >
    <div
      className={`transition-colors ${isActive ? 'text-blue-400' : 'text-gray-500 group-hover:text-gray-400'}`}
      aria-hidden="true"
    >
      {icon}
    </div>
    <span className="flex-1 text-left font-medium">{label}</span>
    {suffix}
  </button>
);
