import React, { Suspense } from 'react';
import type { ReviewTask, Feedback } from '../../types';
import { TaskList, TaskListSkeleton } from './TaskList';
import { FeedbackList } from './FeedbackList';
import { ICONS } from '../../constants/icons';

interface ReviewSidebarProps {
  sidebarTab: 'tasks' | 'feedback';
  tasks: ReviewTask[];
  feedbacks: Feedback[];
  selectedTaskId: string | null;
  selectedFeedbackId: string | null;
  isTasksLoading: boolean;
  isTasksFetching: boolean;
  isFeedbacksLoading: boolean;
  onSidebarTabChange: (tab: 'tasks' | 'feedback') => void;
  onSelectTask: (taskId: string | null) => void;
  onSelectFeedback: (feedbackId: string | null) => void;
  onOpenExportModal: () => void;
}

export const ReviewSidebar: React.FC<ReviewSidebarProps> = ({
  sidebarTab,
  tasks,
  feedbacks,
  selectedTaskId,
  selectedFeedbackId,
  isTasksLoading,
  isTasksFetching,
  isFeedbacksLoading,
  onSidebarTabChange,
  onSelectTask,
  onSelectFeedback,
  onOpenExportModal,
}) => {
  const handleTabChange = (tab: 'tasks' | 'feedback') => {
    onSidebarTabChange(tab);
    if (tab === 'tasks') {
      onSelectFeedback(null);
      if (tasks.length > 0 && !selectedTaskId) {
        onSelectTask(tasks[0].id);
      }
    } else {
      onSelectTask(null);
      if (feedbacks.length > 0 && !selectedFeedbackId) {
        onSelectFeedback(feedbacks[0].id);
      }
    }
  };

  return (
    <div className="border-border bg-bg-secondary/30 flex w-[300px] flex-col border-r">
      <div className="border-border bg-bg-secondary/50 border-b p-3">
        <div className="mb-3 flex gap-2">
          <button
            onClick={onOpenExportModal}
            className="bg-bg-tertiary hover:bg-bg-tertiary/80 text-text-secondary hover:text-text-primary border-border/50 flex flex-1 items-center justify-center gap-1.5 rounded border py-1.5 text-[10px] font-medium transition-all"
            title="Export Review"
          >
            <ICONS.ACTION_EXPORT size={12} />
            Export
          </button>
        </div>

        <div className="bg-bg-tertiary border-border/50 flex rounded-md border p-0.5">
          <TabButton
            active={sidebarTab === 'tasks'}
            onClick={() => handleTabChange('tasks')}
            icon={ICONS.TAB_DESCRIPTION}
            label="Tasks"
            count={tasks.length}
          />
          <div className="bg-border/50 mx-0.5 h-4 w-px self-center" />
          <TabButton
            active={sidebarTab === 'feedback'}
            onClick={() => handleTabChange('feedback')}
            icon={ICONS.ICON_FEEDBACK}
            label="Feedback"
            count={feedbacks.length}
          />
        </div>
      </div>

      {sidebarTab === 'tasks' && (
        <Suspense fallback={<TaskListSkeleton />}>
          <TaskList
            tasks={tasks}
            selectedTaskId={selectedTaskId}
            onSelectTask={onSelectTask}
            isLoading={isTasksLoading}
            isFetching={isTasksFetching}
          />
        </Suspense>
      )}

      {sidebarTab === 'feedback' && (
        <FeedbackList
          feedbacks={feedbacks}
          selectedFeedbackId={selectedFeedbackId}
          onSelectFeedback={onSelectFeedback}
          isLoading={isFeedbacksLoading}
        />
      )}
    </div>
  );
};

interface TabButtonProps {
  active: boolean;
  onClick: () => void;
  icon: React.ElementType;
  label: string;
  count: number;
}

const TabButton: React.FC<TabButtonProps> = ({ active, onClick, icon: Icon, label, count }) => (
  <button
    onClick={onClick}
    className={`flex flex-1 items-center justify-center gap-1.5 rounded-[3px] py-1.5 text-[10px] font-medium transition-all ${
      active
        ? 'bg-bg-primary text-text-primary shadow-sm'
        : 'text-text-disabled hover:text-text-secondary'
    }`}
  >
    <Icon size={12} />
    {label}
    {count > 0 && <span className="text-[10px] opacity-60">({count})</span>}
  </button>
);
