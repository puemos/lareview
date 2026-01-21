import React, { useState, useEffect, Suspense, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useAppStore } from '../../store';

import { DiffViewer } from '../DiffViewer/DiffViewer';
import { useReview } from '../../hooks/useReview';
import { useParsedDiff } from '../../hooks/useParsedDiff';
import { useTasks } from '../../hooks/useTasks';
import { useFeedback, useFeedbackComments, useAddComment } from '../../hooks/useFeedback';
import { useRules } from '../../hooks/useRules';
import { FeedbackDetail } from './FeedbackDetail';
import { ReviewSidebar } from './ReviewSidebar';
import { TaskDetail } from './TaskDetail';
import { DiffSkeleton } from './DiffSkeleton';
import { EmptyState } from './EmptyState';
import { ErrorState } from './ErrorState';
import { SelectionModal, ExportFormat } from './SelectionModal';
import { PushToVcsModal } from './PushToGitHubModal';
import { ConfirmationModal } from '../Common/ConfirmationModal';
import { useReviews } from '../../hooks/useReviews';
import { useTauri } from '../../hooks/useTauri';
import { useDelayedLoading } from '../../hooks/useDelayedLoading';
import { useFeedbackFilterConfig } from '../../hooks/useSettings';
import type { ReviewTask, Feedback, ReviewRule, DefaultIssueCategory } from '../../types';
import { ReviewViewSkeleton } from './ReviewViewSkeleton';
import { AddFeedbackModal } from './AddFeedbackModal';
import type { DiffFile } from '../../types';
import { ReviewSummary } from './ReviewSummary';
import type { SidebarTab } from './ReviewSidebar';

export const ReviewView: React.FC = () => {
  const selectedFile = useAppStore(state => state.selectedFile);
  const selectFile = useAppStore(state => state.selectFile);
  const selectedTaskId = useAppStore(state => state.selectedTaskId);
  const selectTask = useAppStore(state => state.selectTask);
  const selectedFeedbackId = useAppStore(state => state.selectedFeedbackId);
  const selectFeedback = useAppStore(state => state.selectFeedback);
  const reviewId = useAppStore(state => state.reviewId);
  const reviewViewMode = useAppStore(state => state.reviewViewMode);
  const setReviewViewMode = useAppStore(state => state.setReviewViewMode);

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
    createFeedback,
    isCreating: isCreatingFeedback,
  } = useFeedback(reviewId);
  const { data: rules = [] } = useRules();
  const { comments, isLoading: isCommentsLoading } = useFeedbackComments(selectedFeedbackId);
  const addCommentMutation = useAddComment();
  const {
    exportReviewMarkdown,
    pushRemoteReview,
    pushRemoteFeedback,
    copyToClipboard,
    getRepoRootForReview,
    getDefaultIssueCategories,
  } = useTauri();
  const { data: allReviews = [] } = useReviews();
  const { config: filterConfig } = useFeedbackFilterConfig();

  // Fetch the local repository root for this review (matched via remote URL)
  const { data: repoRoot } = useQuery<string | null>({
    queryKey: ['repo-root', reviewId],
    queryFn: () => (reviewId ? getRepoRootForReview(reviewId) : Promise.resolve(null)),
    enabled: !!reviewId,
    staleTime: Infinity, // Repo root doesn't change during a session
  });

  const [activeTab, setActiveTab] = useState<'diff' | 'description' | 'diagram'>('description');
  const [sidebarTab, setSidebarTab] = useState<SidebarTab>('tasks');
  const [isModalOpen, setIsModalOpen] = useState(false);
  const [isPushModalOpen, setIsPushModalOpen] = useState(false);
  const [isDeleteFeedbackModalOpen, setIsDeleteFeedbackModalOpen] = useState(false);

  // Feedback Modal State
  const [isAddFeedbackModalOpen, setIsAddFeedbackModalOpen] = useState(false);
  const [addFeedbackContext, setAddFeedbackContext] = useState<{
    type: 'global' | 'line';
    file?: string;
    line?: number;
    side?: 'old' | 'new';
  }>({ type: 'global' });

  useEffect(() => {
    if (tasks.length > 0 && !selectedTaskId && !isTasksLoading && reviewViewMode === 'review' && sidebarTab === 'tasks') {
      selectTask(tasks[0].id);
    }
  }, [tasks, selectedTaskId, isTasksLoading, selectTask, reviewViewMode, sidebarTab]);

  const handleStartReview = () => {
    setReviewViewMode('review');
    setSidebarTab('tasks');
    if (tasks.length > 0) {
      selectTask(tasks[0].id);
    }
  };

  const handleSelectTaskFromSummary = (taskId: string) => {
    selectTask(taskId);
    setReviewViewMode('review');
    setSidebarTab('tasks');
  };

  const handleBackToSummary = () => {
    setReviewViewMode('summary');
    selectTask(null);
    selectFeedback(null);
  };

  const handleSelectFileFromSummary = (fileName: string) => {
    const file = parsedDiff?.files?.find(f => f.name === fileName || f.new_path === fileName);
    if (file) {
      selectFile(file);
      setReviewViewMode('review');
      setSidebarTab('tasks');
    }
  };

  const handleSelectFeedbackFromSummary = (feedbackId: string) => {
    selectFeedback(feedbackId);
    setReviewViewMode('review');
    setSidebarTab('feedback');
  };

  const selectedTask = tasks.find((t: ReviewTask) => t.id === selectedTaskId);
  const selectedFeedback: Feedback | null =
    feedbacks.find((f: Feedback) => f.id === selectedFeedbackId) || null;
  const rulesById = useMemo(() => {
    const map: Record<string, ReviewRule> = {};
    rules.forEach(rule => {
      map[rule.id] = rule;
    });
    return map;
  }, [rules]);

  // Fetch default issue categories for category badge display
  const { data: defaultCategories = [] } = useQuery<DefaultIssueCategory[]>({
    queryKey: ['default-issue-categories'],
    queryFn: getDefaultIssueCategories,
    staleTime: Infinity, // Categories don't change during a session
  });

  const categoriesById = useMemo(() => {
    const map: Record<string, DefaultIssueCategory> = {};
    defaultCategories.forEach(cat => {
      map[cat.id] = cat;
    });
    return map;
  }, [defaultCategories]);

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
    setIsDeleteFeedbackModalOpen(true);
  };

  const confirmDeleteFeedback = () => {
    if (selectedFeedbackId) {
      deleteFeedback({ feedbackId: selectedFeedbackId });
      selectFeedback(null);
      setIsDeleteFeedbackModalOpen(false);
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
  const remoteProviderName =
    currentReview?.source?.type === 'gitlab_mr'
      ? 'GitLab'
      : currentReview?.source?.type === 'github_pr' ||
          (currentReview?.source?.type as string) === 'git_hub_pr'
        ? 'GitHub'
        : null;

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
      const url = await pushRemoteReview(reviewId, selectedTasks, selectedFeedbacks);
      return url;
    }
  };

  const handlePushFeedbackToRemote = () => {
    if (!selectedFeedbackId) return;
    setIsPushModalOpen(true);
  };

  const handleConfirmPush = async () => {
    if (!selectedFeedbackId) return;
    return await pushRemoteFeedback(selectedFeedbackId);
  };

  const handleAddGlobalFeedback = () => {
    setAddFeedbackContext({ type: 'global' });
    setIsAddFeedbackModalOpen(true);
  };

  const handleAddLineFeedback = (file: DiffFile, line: number, side: 'old' | 'new') => {
    setAddFeedbackContext({
      type: 'line',
      file: file.name || file.new_path || 'unknown',
      line,
      side,
    });
    setIsAddFeedbackModalOpen(true);
  };

  const handleCreateFeedback = (
    title: string,
    impact: 'blocking' | 'nice_to_have' | 'nitpick',
    content: string
  ) => {
    if (!reviewId) return;

    createFeedback({
      review_id: reviewId,
      title,
      content,
      impact,
      file_path: addFeedbackContext.type === 'line' ? addFeedbackContext.file : undefined,
      line_number: addFeedbackContext.type === 'line' ? addFeedbackContext.line : undefined,
      side: addFeedbackContext.type === 'line' ? addFeedbackContext.side : undefined,
    });

    setIsAddFeedbackModalOpen(false);
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
    return <ReviewViewSkeleton mode={reviewViewMode} />;
  }

  if (reviewError) {
    return <ErrorState error={reviewError} onRetry={handleRetry} />;
  }

  // Summary mode - full width, no sidebar
  if (reviewViewMode === 'summary') {
    return (
      <div className="bg-bg-primary flex h-full flex-col">
        <Suspense fallback={<DiffSkeleton />}>
          {diffError ? (
            <ErrorState
              error={diffError instanceof Error ? diffError : new Error(String(diffError))}
              onRetry={handleRetry}
            />
          ) : (
            <ReviewSummary
              runId={runId ?? undefined}
              tasks={tasks}
              feedbacks={feedbacks}
              parsedDiff={parsedDiff ?? null}
              review={currentReview ?? null}
              onSelectFeedback={handleSelectFeedbackFromSummary}
              onSelectFile={handleSelectFileFromSummary}
              onSelectTask={handleSelectTaskFromSummary}
              onStartReview={handleStartReview}
            />
          )}
        </Suspense>

        <SelectionModal
          isOpen={isModalOpen}
          onClose={() => setIsModalOpen(false)}
          onConfirm={handleExport}
          tasks={tasks}
          feedbacks={feedbacks}
          remoteProviderName={remoteProviderName}
        />
      </div>
    );
  }

  // Review mode - sidebar + content
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
        confidenceThreshold={filterConfig.confidenceThreshold}
        onSidebarTabChange={setSidebarTab}
        onSelectTask={selectTask}
        onSelectFeedback={selectFeedback}
        onOpenExportModal={() => setIsModalOpen(true)}
        onAddGlobalFeedback={handleAddGlobalFeedback}
        onBackToSummary={handleBackToSummary}
        rulesById={rulesById}
        categoriesById={categoriesById}
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
              rulesById={rulesById}
              categoriesById={categoriesById}
              comments={isCommentsLoading ? [] : comments}
              onUpdateStatus={handleFeedbackStatusChange}
              onUpdateImpact={handleFeedbackImpactChange}
              onDelete={handleDeleteFeedback}
              onAddComment={handleAddComment}
              onPushToRemote={handlePushFeedbackToRemote}
              remoteProviderName={remoteProviderName}
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
                  onAddFeedback={handleAddLineFeedback}
                  repoRoot={repoRoot}
                />
              ) : parsedDiff ? (
                <div className="flex flex-1 flex-col">
                  <div className="border-border bg-bg-primary h-10 border-b" />
                  <div className="relative flex-1">
                    <DiffViewer
                      files={parsedDiff.files || []}
                      selectedFile={selectedFile}
                      onSelectFile={selectFile}
                      onAddFeedback={handleAddLineFeedback}
                      repoRoot={repoRoot}
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
        remoteProviderName={remoteProviderName}
      />

      <PushToVcsModal
        isOpen={isPushModalOpen}
        providerName={remoteProviderName}
        onClose={() => setIsPushModalOpen(false)}
        onConfirm={handleConfirmPush}
      />

      <AddFeedbackModal
        isOpen={isAddFeedbackModalOpen}
        onClose={() => setIsAddFeedbackModalOpen(false)}
        onAdd={handleCreateFeedback}
        context={addFeedbackContext}
        isAdding={isCreatingFeedback}
      />

      <ConfirmationModal
        isOpen={isDeleteFeedbackModalOpen}
        onClose={() => setIsDeleteFeedbackModalOpen(false)}
        onConfirm={confirmDeleteFeedback}
        title="Delete Feedback"
        message="Are you sure you want to delete this feedback item? This action cannot be undone."
        confirmLabel="Delete"
        confirmVariant="danger"
      />
    </div>
  );
};
