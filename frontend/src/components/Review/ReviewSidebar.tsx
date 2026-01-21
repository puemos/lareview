import React, { Suspense } from 'react';
import { motion } from 'framer-motion';
import { ArrowLeft } from '@phosphor-icons/react';
import { SHARED_LAYOUT_TRANSITION } from '../../constants/animations';
import type { ReviewTask, Feedback, ReviewRule, DefaultIssueCategory } from '../../types';
import { TaskList, TaskListSkeleton } from './TaskList';
import { FeedbackList } from './FeedbackList';
import { ICONS } from '../../constants/icons';

export type SidebarTab = 'tasks' | 'feedback';

interface ReviewSidebarProps {
  sidebarTab: SidebarTab;
  tasks: ReviewTask[];
  feedbacks: Feedback[];
  rulesById: Record<string, ReviewRule>;
  categoriesById: Record<string, DefaultIssueCategory>;
  selectedTaskId: string | null;
  selectedFeedbackId: string | null;
  isTasksLoading: boolean;
  isTasksFetching: boolean;
  isFeedbacksLoading: boolean;
  confidenceThreshold?: number | null;
  onSidebarTabChange: (tab: SidebarTab) => void;
  onSelectTask: (taskId: string | null) => void;
  onSelectFeedback: (feedbackId: string | null) => void;
  onOpenExportModal: () => void;
  onAddGlobalFeedback: () => void;
  onBackToSummary: () => void;
}

export const ReviewSidebar: React.FC<ReviewSidebarProps> = ({
  sidebarTab,
  tasks,
  feedbacks,
  rulesById,
  categoriesById,
  selectedTaskId,
  selectedFeedbackId,
  isTasksLoading,
  isTasksFetching,
  isFeedbacksLoading,
  confidenceThreshold,
  onSidebarTabChange,
  onSelectTask,
  onSelectFeedback,
  onOpenExportModal,
  onAddGlobalFeedback,
  onBackToSummary,
}) => {
  const handleTabChange = (tab: SidebarTab) => {
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
        {/* Back to Summary link */}
        <button
          onClick={onBackToSummary}
          className="text-text-secondary hover:text-text-primary mb-3 flex w-full items-center gap-1.5 text-xs transition-colors"
        >
          <ArrowLeft size={14} weight="bold" />
          <span>Summary</span>
        </button>

        <div className="mb-3 flex">
          <div
            className={`flex items-center overflow-hidden transition-all duration-300 ease-in-out ${
              sidebarTab === 'feedback' ? 'mr-2 flex-1 opacity-100' : 'mr-0 flex-[0_0_0] opacity-0'
            }`}
          >
            <button
              onClick={onAddGlobalFeedback}
              className="bg-brand text-brand-fg border-brand hover:bg-brand/90 flex w-full cursor-pointer items-center justify-center gap-1.5 rounded border px-3 py-1.5 text-[10px] font-medium whitespace-nowrap transition-all"
              title="Add Global Feedback"
            >
              <ICONS.ICON_FEEDBACK size={12} weight="bold" />
              Add Feedback
            </button>
          </div>
          <button
            onClick={onOpenExportModal}
            className="bg-bg-tertiary hover:bg-bg-tertiary/80 text-text-secondary hover:text-text-primary border-border/50 flex flex-1 cursor-pointer items-center justify-center gap-1.5 overflow-hidden rounded border py-1.5 text-[10px] font-medium whitespace-nowrap transition-all"
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
          rulesById={rulesById}
          categoriesById={categoriesById}
          selectedFeedbackId={selectedFeedbackId}
          onSelectFeedback={onSelectFeedback}
          isLoading={isFeedbacksLoading}
          confidenceThreshold={confidenceThreshold}
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
  count?: number;
}

const TabButton: React.FC<TabButtonProps> = ({ active, onClick, icon: Icon, label, count }) => (
  <button
    onClick={onClick}
    className={`relative flex flex-1 cursor-pointer items-center justify-center gap-1.5 rounded-[3px] py-1.5 text-[10px] font-medium transition-colors ${
      active ? 'text-text-primary' : 'text-text-disabled hover:text-text-secondary'
    }`}
  >
    {active && (
      <motion.div
        layoutId="sidebar-tab-bg"
        className="bg-bg-primary absolute inset-0 z-0 rounded-[3px] shadow-sm"
        transition={SHARED_LAYOUT_TRANSITION}
      />
    )}
    <div className="relative z-10 flex items-center gap-1.5">
      <Icon size={12} />
      {label}
      {count !== undefined && count > 0 && <span className="text-[10px] opacity-60">({count})</span>}
    </div>
  </button>
);
