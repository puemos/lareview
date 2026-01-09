import React, { useState, useEffect, Suspense } from 'react';
import { useAppStore } from '../../store';
import { DiffViewer } from '../DiffViewer/DiffViewer';
import { useReview } from '../../hooks/useReview';
import { useParsedDiff } from '../../hooks/useParsedDiff';
import { useTasks } from '../../hooks/useTasks';
import { useFeedback, useFeedbackComments, useAddComment } from '../../hooks/useFeedback';
import { FeedbackDetail } from './FeedbackDetail';
import { ReviewSidebar } from './ReviewSidebar';
import { TaskDetail } from './TaskDetail';
import { DiffSkeleton } from './DiffSkeleton';
import { EmptyState } from './EmptyState';
import { ErrorState } from './ErrorState';
import { SelectionModal, ExportFormat } from './SelectionModal';
import { PushToGitHubModal } from './PushToGitHubModal';
import { useReviews } from '../../hooks/useReviews';
import { useTauri } from '../../hooks/useTauri';
import { useDelayedLoading } from '../../hooks/useDelayedLoading';
import type { ReviewTask, Feedback } from '../../types';
import { ReviewViewSkeleton } from './ReviewViewSkeleton';

export const ReviewView: React.FC = () => {
  const selectedFile = useAppStore(state => state.selectedFile);
  const selectFile = useAppStore(state => state.selectFile);
  const selectedTaskId = useAppStore(state => state.selectedTaskId);
  const selectTask = useAppStore(state => state.selectTask);
  const selectedFeedbackId = useAppStore(state => state.selectedFeedbackId);
  const selectFeedback = useAppStore(state => state.selectFeedback);
  const reviewId = useAppStore(state => state.reviewId);

  const { runId, firstRun, error: reviewError, isLoading: isReviewLoading } = useReview(reviewId);
  const { data: parsedDiff, error: diffError } = useParsedDiff(runId, firstRun?.diff_text ?? null);
  const {
    data: tasks = [],
    isLoading: isTasksLoading,
    isFetching: isTasksFetching,
    updateTaskStatus,
    isUpdatingStatus,
  } = useTasks(runId);
  const {
    data: feedbacks = [],
    isLoading: isFeedbacksLoading,
    updateStatus,
    isUpdatingStatus: isUpdatingFeedbackStatus,
    updateImpact,
    isUpdatingImpact: isUpdatingFeedbackImpact,
    deleteFeedback,
  } = useFeedback(reviewId);
  const { comments, isLoading: isCommentsLoading } = useFeedbackComments(selectedFeedbackId);
  const addCommentMutation = useAddComment();
  const { exportReviewMarkdown, pushGitHubReview, pushGitHubFeedback, copyToClipboard } =
    useTauri();
  const { data: allReviews = [] } = useReviews();

  const [activeTab, setActiveTab] = useState<'diff' | 'description' | 'diagram'>('description');
  const [sidebarTab, setSidebarTab] = useState<'tasks' | 'feedback'>('tasks');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isPushModalOpen, setIsPushModalOpen] = useState(false);

  useEffect(() => {
    if (tasks.length > 0 && !selectedTaskId && !isTasksLoading && sidebarTab === 'tasks') {
      selectTask(tasks[0].id);
    }
  }, [tasks, selectedTaskId, isTasksLoading, selectTask, sidebarTab]);

  const selectedTask = tasks.find((t: ReviewTask) => t.id === selectedTaskId);
  const selectedFeedback: Feedback | null =
    feedbacks.find((f: Feedback) => f.id === selectedFeedbackId) || null;

  const handleStatusChange = (status: string) => {
    if (selectedTask) {
      updateTaskStatus({
        taskId: selectedTask.id,
        status: status as typeof selectedTask.status,
      });
    }
  };

  const handleFeedbackStatusChange = (status: Feedback['status']) => {
    if (selectedFeedbackId) {
      updateStatus({ feedbackId: selectedFeedbackId, status });
    }
  };

  const handleFeedbackImpactChange = (impact: Feedback['impact']) => {
    if (selectedFeedbackId) {
      updateImpact({ feedbackId: selectedFeedbackId, impact });
    }
  };

  const handleDeleteFeedback = () => {
    if (selectedFeedbackId && window.confirm('Are you sure you want to delete this feedback?')) {
      deleteFeedback({ feedbackId: selectedFeedbackId });
      selectFeedback(null);
    }
  };

  const handleAddComment = (body: string) => {
    if (selectedFeedbackId) {
      addCommentMutation.mutate({ feedbackId: selectedFeedbackId, body });
    }
  };

  const handleRetry = () => {
    window.location.reload();
  };

  const currentReview = allReviews.find(r => r.id === reviewId);
  const isGitHubReview =
    currentReview?.source?.type === 'github_pr' ||
    (currentReview?.source?.type as string) === 'git_hub_pr';

  console.log('Review Debug:', {
    reviewId,
    found: !!currentReview,
    sourceType: currentReview?.source?.type,
    isGitHubReview,
  });

  const handleExport = async (
    format: ExportFormat,
    selectedTasks: string[],
    selectedFeedbacks: string[]
  ) => {
    if (!reviewId) return;

    if (format === 'markdown') {
      const markdown = await exportReviewMarkdown(reviewId, selectedTasks, selectedFeedbacks);
      await copyToClipboard(markdown);
      // Success alert is handled in SelectionModal or implied by modal closing
      // but we return nothing to keep it generic
    } else {
      const url = await pushGitHubReview(reviewId, selectedTasks, selectedFeedbacks);
      return url;
    }
  };

  const handlePushFeedbackToGitHub = () => {
    if (!selectedFeedbackId) return;
    setIsPushModalOpen(true);
  };

  const handleConfirmPush = async () => {
    if (!selectedFeedbackId) return;
    return await pushGitHubFeedback(selectedFeedbackId);
  };

  // Use delayed loading to prevent flashing
  const shouldShowSkeleton = useDelayedLoading(
    isReviewLoading || (!!reviewId && !runId && !reviewError)
  );

  if (!reviewId) {
    return (
      <EmptyState
        title="No Review Selected"
        description="Select a review from the sidebar to view details"
      />
    );
  }

  if (shouldShowSkeleton) {
    return <ReviewViewSkeleton />;
  }

  if (reviewError) {
    return <ErrorState error={reviewError} onRetry={handleRetry} />;
  }

  return (
    <div className="bg-bg-primary flex h-full">
      <ReviewSidebar
        sidebarTab={sidebarTab}
        tasks={tasks}
        feedbacks={feedbacks}
        selectedTaskId={selectedTaskId}
        selectedFeedbackId={selectedFeedbackId}
        isTasksLoading={isTasksLoading}
        isTasksFetching={isTasksFetching}
        isFeedbacksLoading={isFeedbacksLoading}
        onSidebarTabChange={setSidebarTab}
        onSelectTask={selectTask}
        onSelectFeedback={selectFeedback}
        onOpenExportModal={() => setIsModalOpen(true)}
      />

      <div className="bg-bg-primary relative flex min-w-0 flex-1 flex-col">
        <Suspense fallback={<DiffSkeleton />}>
          {diffError ? (
            <ErrorState
              error={diffError instanceof Error ? diffError : new Error(String(diffError))}
              onRetry={handleRetry}
            />
          ) : sidebarTab === 'feedback' ? (
            <FeedbackDetail
              feedback={selectedFeedback}
              comments={isCommentsLoading ? [] : comments}
              onUpdateStatus={handleFeedbackStatusChange}
              onUpdateImpact={handleFeedbackImpactChange}
              onDelete={handleDeleteFeedback}
              onAddComment={handleAddComment}
              onPushToGitHub={handlePushFeedbackToGitHub}
              isGitHubReview={isGitHubReview}
              isUpdatingStatus={isUpdatingFeedbackStatus}
              isUpdatingImpact={isUpdatingFeedbackImpact}
              isAddingComment={addCommentMutation.isPending}
            />
          ) : (
            <>
              {selectedTask && parsedDiff ? (
                <TaskDetail
                  task={selectedTask}
                  parsedDiff={parsedDiff}
                  selectedFile={selectedFile}
                  onSelectFile={selectFile}
                  highlightedHunks={selectedTask.diff_refs.flatMap(ref =>
                    ref.hunks.map(h => ({
                      file: ref.file,
                      oldStart: h.old_start,
                      oldLines: h.old_lines,
                      newStart: h.new_start,
                      newLines: h.new_lines,
                    }))
                  )}
                  activeTab={activeTab}
                  onTabChange={setActiveTab}
                  onStatusChange={handleStatusChange}
                  isUpdatingStatus={isUpdatingStatus}
                />
              ) : parsedDiff ? (
                <div className="flex flex-1 flex-col">
                  <div className="border-border bg-bg-primary h-10 border-b" />
                  <div className="relative flex-1">
                    <DiffViewer
                      files={parsedDiff.files || []}
                      selectedFile={selectedFile}
                      onSelectFile={selectFile}
                    />
                  </div>
                </div>
              ) : (
                <DiffSkeleton />
              )}
            </>
          )}
        </Suspense>
      </div>
      <SelectionModal
        isOpen={isModalOpen}
        onClose={() => setIsModalOpen(false)}
        onConfirm={handleExport}
        tasks={tasks}
        feedbacks={feedbacks}
        isGitHubAvailable={isGitHubReview}
      />
      
      <PushToGitHubModal
        isOpen={isPushModalOpen}
        onClose={() => setIsPushModalOpen(false)}
        onConfirm={handleConfirmPush}
      />
    </div>
  );
};
