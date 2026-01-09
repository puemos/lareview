import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { 
  SPRING_TIGHT, 
  SHARED_LAYOUT_TRANSITION, 
  WHILE_TAP_SCALE 
} from '../../constants/animations';
import type { ReviewTask } from '../../types';
import { ICONS } from '../../constants/icons';

interface TaskListProps {
  tasks: ReviewTask[];
  selectedTaskId: string | null;
  onSelectTask: (taskId: string) => void;
  isLoading: boolean;
  isFetching: boolean;
}

import { Tooltip } from '../Common/Tooltip';

export const TaskList: React.FC<TaskListProps> = ({
  tasks,
  selectedTaskId,
  onSelectTask,
  isLoading,
  isFetching,
}) => {
  if (isLoading || isFetching) {
    return <TaskListSkeleton />;
  }

  if (tasks.length === 0) {
    return (
      <div className="text-text-disabled p-8 text-center text-xs opacity-50">No tasks found</div>
    );
  }

  const getRiskIcon = (risk: string) => {
    switch (risk) {
      case 'low':
        return ICONS.RISK_LOW;
      case 'medium':
        return ICONS.RISK_MEDIUM;
      case 'high':
        return ICONS.RISK_HIGH;
      default:
        return ICONS.RISK_LOW;
    }
  };

  const getRiskColor = (risk: string) => {
    switch (risk) {
      case 'low':
        return 'text-risk-low';
      case 'medium':
        return 'text-risk-medium';
      case 'high':
        return 'text-risk-high';
      default:
        return 'text-risk-low';
    }
  };

  const containerVariants = {
    hidden: { opacity: 0 },
    visible: {
      opacity: 1,
      transition: {
        staggerChildren: 0.03,
      },
    },
  };

  const itemVariants = {
    hidden: { opacity: 0, y: 8, scale: 0.98 },
    visible: {
      opacity: 1,
      y: 0,
      scale: 1,
      transition: SPRING_TIGHT,
    },
  } as const;

  return (
    <motion.div
      variants={containerVariants}
      initial="hidden"
      animate="visible"
      className="custom-scrollbar flex-1 overflow-y-auto"
    >
      <AnimatePresence initial={false}>
        {tasks.map((task: ReviewTask) => {
          const RiskIcon = getRiskIcon(task.stats.risk);
          const riskColor = getRiskColor(task.stats.risk);

          return (
            <motion.button
              key={task.id}
              variants={itemVariants}
              whileTap={WHILE_TAP_SCALE}
              onClick={() => onSelectTask(task.id)}
              className={`group border-border/50 hover:bg-bg-secondary/80 relative w-full border-b px-4 py-3 text-left transition-colors ${
                selectedTaskId === task.id ? 'bg-bg-secondary shadow-inner' : ''
              }`}
            >
              {selectedTaskId === task.id && (
                <motion.div
                  layoutId="active-task-indicator"
                  className="bg-brand absolute top-0 bottom-0 left-0 w-[2px]"
                  transition={SHARED_LAYOUT_TRANSITION}
                />
              )}
            <div className="flex w-full min-w-0 items-center gap-2.5">
              <div className="flex-shrink-0">
                <Tooltip content={`Risk: ${task.stats.risk.toUpperCase()}`}>
                  <div className="cursor-help">
                    <RiskIcon size={14} className={riskColor} />
                  </div>
                </Tooltip>
              </div>
              <h3
                className={`flex-1 truncate text-xs leading-relaxed font-medium ${
                  selectedTaskId === task.id
                    ? 'text-text-primary'
                    : 'text-text-secondary group-hover:text-text-primary'
                } ${task.status === 'done' ? 'text-text-disabled line-through opacity-50' : ''}`}
              >
                {task.title}
              </h3>
            </div>
            </motion.button>
          );
        })}
      </AnimatePresence>
    </motion.div>
  );
};

export function TaskListSkeleton() {
  return (
    <div className="custom-scrollbar flex-1 overflow-y-auto">
      {[1, 2, 3, 4, 5].map(i => (
        <div key={i} className="border-border/50 animate-pulse border-b px-4 py-3">
          <div className="flex items-start gap-2.5">
            <div className="bg-bg-tertiary mt-0.5 h-3.5 w-3.5 rounded-full" />
            <div className="flex-1 space-y-1.5">
              <div className="bg-bg-tertiary h-3 w-3/4 rounded" />
              <div className="bg-bg-tertiary h-2 w-1/2 rounded" />
            </div>
          </div>
        </div>
      ))}
    </div>
  );
}
