import React from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import { SHARED_LAYOUT_TRANSITION, CONTENT_TRANSITION } from '../../constants/animations';
import ReactMarkdown from 'react-markdown';
import { Mermaid } from '../Common/Mermaid';
import { DiffViewer } from '../DiffViewer/DiffViewer';
import { Select } from '../Common/Select';
import type { ReviewTask, ParsedDiff, DiffFile, DiffRef, HunkRef } from '../../types';
import { ICONS } from '../../constants/icons';
import { Tooltip } from '../Common/Tooltip';

interface TaskDetailProps {
  task: ReviewTask | null;
  parsedDiff: ParsedDiff | null | undefined;
  selectedFile: DiffFile | null;
  onSelectFile: (file: DiffFile | null) => void;
  highlightedHunks: Array<{
    file: string;
    oldStart: number;
    oldLines: number;
    newStart: number;
    newLines: number;
  }>;
  activeTab: 'diff' | 'description' | 'diagram';
  onTabChange: (tab: 'diff' | 'description' | 'diagram') => void;
  onStatusChange?: (status: string) => void;
  isUpdatingStatus?: boolean;
  onAddFeedback?: (file: DiffFile, line: number, side: 'old' | 'new') => void;
}

type IconComponent = React.ComponentType<{ size: number; className?: string }>;

export const TaskDetail: React.FC<TaskDetailProps> = ({
  task,
  parsedDiff,
  selectedFile,
  onSelectFile,
  activeTab,
  onTabChange,
  onStatusChange,
  isUpdatingStatus,
  onAddFeedback,
}) => {
  if (!task) {
    return (
      <div className="text-text-disabled flex flex-1 items-center justify-center">
        <div className="space-y-3 text-center opacity-50">
          <div className="bg-bg-secondary mx-auto mb-2 flex h-12 w-12 items-center justify-center rounded-xl">
            <ICONS.ICON_PLAN size={24} />
          </div>
          <p className="text-xs font-medium">Select a task to view details</p>
        </div>
      </div>
    );
  }

  const STATUS_OPTIONS = [
    {
      value: 'todo',
      label: 'Todo',
      icon: ICONS.STATUS_TODO,
      color: 'text-status-todo',
    },
    {
      value: 'in_progress',
      label: 'In Progress',
      icon: ICONS.STATUS_IN_PROGRESS,
      color: 'text-status-in_progress',
    },
    {
      value: 'done',
      label: 'Done',
      icon: ICONS.STATUS_DONE,
      color: 'text-status-done',
    },
    {
      value: 'ignored',
      label: 'Ignored',
      icon: ICONS.STATUS_IGNORED,
      color: 'text-status-ignored',
    },
  ];

  const filteredFiles = React.useMemo(() => {
    return (parsedDiff?.files || []).filter(file =>
      task.diff_refs.some(ref => ref.file === file.name)
    );
  }, [parsedDiff?.files, task.diff_refs]);

  React.useEffect(() => {
    if (filteredFiles.length > 0) {
      if (activeTab === 'diff' || !selectedFile) {
        onSelectFile(filteredFiles[0]);
      }
    }
  }, [task.id, activeTab, filteredFiles, onSelectFile, selectedFile]);

  const highlightedHunks = React.useMemo(
    () =>
      task.diff_refs.flatMap((ref: DiffRef) =>
        ref.hunks.map((h: HunkRef) => ({
          file: ref.file,
          oldStart: h.old_start,
          oldLines: h.old_lines,
          newStart: h.new_start,
          newLines: h.new_lines,
        }))
      ),
    [task.diff_refs]
  );

  const diffStats = React.useMemo(() => {
    let additions = 0;
    let deletions = 0;
    for (const file of filteredFiles) {
      for (const hunk of file.hunks) {
        additions += hunk.new_lines;
        deletions += hunk.old_lines;
      }
    }
    return { additions, deletions };
  }, [filteredFiles]);

  // ... (existing imports)

  // ... (inside TaskDetail component)

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

  const RiskIcon = getRiskIcon(task.stats.risk);
  const riskColor = getRiskColor(task.stats.risk);

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <div className="border-border bg-bg-secondary/50 z-10 border-b px-4 py-3">
        <div className="mb-3 flex items-center justify-between gap-3">
          <Tooltip content={`Risk Level: ${task.stats.risk.toUpperCase()}`}>
            <div
              className={`bg-bg-surface flex items-center gap-1.5 rounded-full px-2 py-1 text-[10px] font-medium tracking-wider ${riskColor} border-border/50 border`}
            >
              <RiskIcon size={12} weight="fill" />
              {task.stats.risk}
            </div>
          </Tooltip>
          <h2 className="text-text-primary hover:text-brand flex-1 cursor-pointer truncate text-sm font-medium">
            {task.title}
          </h2>
        </div>

        <div className="flex w-full items-center justify-between gap-2">
          <div className="bg-bg-tertiary/50 border-border/50 flex items-center rounded border">
            <TabButton
              active={activeTab === 'description'}
              onClick={() => onTabChange('description')}
              icon={ICONS.TAB_DESCRIPTION as IconComponent}
              label="Analysis"
            />
            <div className="bg-border/50 mx-0.5 h-3 w-px" />
            <TabButton
              active={activeTab === 'diff'}
              onClick={() => onTabChange('diff')}
              icon={ICONS.TAB_CHANGES as IconComponent}
              label={
                <span className="flex items-center gap-1.5">
                  Changes
                  {diffStats.additions > 0 || diffStats.deletions > 0 ? (
                    <span className="font-mono text-[10px] opacity-70">
                      +{diffStats.additions}/-{diffStats.deletions}
                    </span>
                  ) : null}
                </span>
              }
            />
            {task.diagram && (
              <>
                <div className="bg-border/50 mx-0.5 h-3 w-px" />
                <TabButton
                  active={activeTab === 'diagram'}
                  onClick={() => onTabChange('diagram')}
                  icon={ICONS.TAB_DIAGRAM as IconComponent}
                  label="Diagram"
                />
              </>
            )}
          </div>
          <Select
            value={task.status}
            onChange={value => onStatusChange?.(value)}
            options={STATUS_OPTIONS}
            disabled={isUpdatingStatus}
          />
        </div>
      </div>

      <div className="relative flex-1 overflow-hidden">
        <AnimatePresence mode="wait">
          <motion.div
            key={activeTab}
            initial={{ opacity: 0, y: 4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -2 }}
            transition={CONTENT_TRANSITION}
            className="absolute inset-0"
          >
            {activeTab === 'diff' && (
              <DiffViewer
                files={filteredFiles}
                selectedFile={selectedFile}
                onSelectFile={onSelectFile}
                highlightedHunks={highlightedHunks}
                onAddFeedback={onAddFeedback}
              />
            )}

            {activeTab === 'description' && (
              <div className="custom-scrollbar h-full overflow-y-auto p-8">
                <div className="animate-fade-in mx-auto max-w-3xl space-y-6">
                  <TaskDescription description={task.description} />

                  {task.insight && <TaskInsight insight={task.insight} />}

                  <TaskFiles files={task.files} />
                </div>
              </div>
            )}

            {activeTab === 'diagram' && task.diagram && (
              <div className="custom-scrollbar bg-bg-secondary/20 flex h-full flex-col items-center overflow-y-auto p-8">
                <div className="animate-fade-in w-full max-w-4xl">
                  <Mermaid chart={task.diagram} className="border-border border shadow-sm" />
                </div>
              </div>
            )}
          </motion.div>
        </AnimatePresence>
      </div>
    </div>
  );
};

const TabButton: React.FC<{
  active: boolean;
  onClick: () => void;
  icon: IconComponent;
  label: React.ReactNode;
}> = ({ active, onClick, icon: Icon, label }) => (
  <button
    onClick={onClick}
    className={`relative flex items-center gap-1.5 rounded-[3px] px-3 py-1.5 text-[11px] font-medium transition-colors ${
      active ? 'text-text-primary' : 'text-text-disabled hover:text-text-secondary'
    }`}
  >
    {active && (
      <motion.div
        layoutId="active-tab-bg"
        className="bg-bg-primary absolute inset-0 z-0 rounded-[3px] shadow-sm"
        transition={SHARED_LAYOUT_TRANSITION}
      />
    )}
    <div className="relative z-10 flex items-center gap-1.5">
      <Icon
        size={12}
        className={
          active ? 'text-text-primary' : 'text-text-disabled group-hover:text-text-secondary'
        }
      />
      {label}
    </div>
  </button>
);

interface CodeProps {
  children?: React.ReactNode;
  className?: string;
  node?: unknown;
}

const TaskDescription: React.FC<{ description: string }> = ({ description }) => {
  const CodeBlock = ({ children, className }: CodeProps) => {
    const match = /language-(\w+)/.exec(className || '');
    return match && match[1] === 'mermaid' ? (
      <Mermaid chart={String(children || '').replace(/\n$/, '')} className="my-4" />
    ) : (
      <code className={className}>{children}</code>
    );
  };

  return (
    <div className="prose prose-invert prose-sm text-text-secondary max-w-none leading-relaxed">
      <ReactMarkdown
        components={{
          code: CodeBlock,
        }}
      >
        {description}
      </ReactMarkdown>
    </div>
  );
};

const TaskInsight: React.FC<{ insight: string }> = ({ insight }) => {
  const CodeBlock = ({ children, className }: CodeProps) => {
    const match = /language-(\w+)/.exec(className || '');
    return match && match[1] === 'mermaid' ? (
      <Mermaid chart={String(children || '').replace(/\n$/, '')} className="my-4" />
    ) : (
      <code className={className}>{children}</code>
    );
  };

  return (
    <div className="bg-brand/5 border-brand/10 rounded-lg border p-4">
      <h4 className="text-brand mb-2 flex items-center gap-2 text-xs font-bold">
        <ICONS.VIEW_GENERATE size={12} />
        Insight
      </h4>
      <div className="prose prose-invert prose-sm text-text-secondary max-w-none leading-normal">
        <ReactMarkdown
          components={{
            code: CodeBlock,
          }}
        >
          {insight}
        </ReactMarkdown>
      </div>
    </div>
  );
};

const TaskFiles: React.FC<{ files: string[] }> = ({ files }) => (
  <div className="border-border mt-8 border-t pt-4">
    <div className="flex flex-wrap gap-2">
      {files.map(file => (
        <span
          key={file}
          className="bg-bg-secondary text-text-tertiary border-border/50 flex items-center gap-1.5 rounded border px-2 py-1 font-mono text-[10px]"
        >
          <ICONS.ICON_FILES size={10} />
          {file}
        </span>
      ))}
    </div>
  </div>
);
