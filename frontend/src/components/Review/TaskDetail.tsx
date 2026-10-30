import React from 'react';
import ReactMarkdown from 'react-markdown';
import { Mermaid } from '../Common/Mermaid';
import { DiffViewer } from '../DiffViewer/DiffViewer';
import { countDiffChanges } from '../../lib/diff-stats';
import { Select } from '../Common/Select';
import type { ReviewTask, ParsedDiff, DiffFile, DiffRef, HunkRef } from '../../types';
import { ICONS } from '../../constants/icons';

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
  repoRoot?: string | null;
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
  repoRoot,
}) => {
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
      (task?.diff_refs ?? []).some(ref => ref.file === file.name)
    );
  }, [parsedDiff?.files, task?.diff_refs]);

  React.useEffect(() => {
    if (filteredFiles.length > 0) {
      if (activeTab === 'diff' && !filteredFiles.some(file => file.name === selectedFile?.name)) {
        onSelectFile(filteredFiles[0]);
      }
    }
  }, [task?.id, activeTab, filteredFiles, onSelectFile, selectedFile]);

  const highlightedHunks = React.useMemo(
    () =>
      (task?.diff_refs ?? []).flatMap((ref: DiffRef) =>
        ref.hunks.map((h: HunkRef) => ({
          file: ref.file,
          oldStart: h.old_start,
          oldLines: h.old_lines,
          newStart: h.new_start,
          newLines: h.new_lines,
        }))
      ),
    [task?.diff_refs]
  );

  const diffStats = React.useMemo(() => countDiffChanges(filteredFiles), [filteredFiles]);

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

  const RiskIcon = getRiskIcon(task.stats.risk);
  const riskColor = getRiskColor(task.stats.risk);

  return (
    <div className="bg-bg-primary flex min-h-0 min-w-0 flex-1 flex-col">
      <div className="review-detail-header border-border border-b">
        <div className="mb-4 flex flex-wrap items-start justify-between gap-3">
          <div className="min-w-0 flex-1">
            <p className="text-text-tertiary mb-2 text-xs font-medium">Review task</p>
            <h2 className="text-text-primary text-lg leading-snug font-semibold break-words">
              {task.title}
            </h2>
          </div>
          <span
            className={`border-border bg-bg-secondary flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs ${riskColor}`}
          >
            <RiskIcon size={14} />
            {task.stats.risk} risk
          </span>
        </div>

        <div className="flex w-full flex-wrap items-center justify-between gap-3">
          <div className="review-detail-tabs">
            <TabButton
              active={activeTab === 'description'}
              onClick={() => onTabChange('description')}
              icon={ICONS.TAB_DESCRIPTION as IconComponent}
              label="Analysis"
            />
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
              <TabButton
                active={activeTab === 'diagram'}
                onClick={() => onTabChange('diagram')}
                icon={ICONS.TAB_DIAGRAM as IconComponent}
                label="Diagram"
              />
            )}
          </div>
          <Select
            aria-label="Task status"
            value={task.status === 'pending' ? 'todo' : task.status}
            onChange={value => onStatusChange?.(value)}
            options={STATUS_OPTIONS}
            disabled={isUpdatingStatus}
          />
        </div>
      </div>

      <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
        <div className="flex min-h-0 flex-1 flex-col">
          {activeTab === 'diff' && (
            <DiffViewer
              files={filteredFiles}
              selectedFile={selectedFile}
              onSelectFile={onSelectFile}
              highlightedHunks={highlightedHunks}
              onAddFeedback={onAddFeedback}
              repoRoot={repoRoot}
            />
          )}

          {activeTab === 'description' && (
            <div className="custom-scrollbar min-h-0 flex-1 overflow-y-auto p-6">
              <div className="mx-auto w-full max-w-3xl space-y-6">
                <TaskDescription description={task.description} />

                {task.insight && <TaskInsight insight={task.insight} />}

                <TaskFiles
                  files={task.files}
                  availableFiles={filteredFiles}
                  onSelect={file => {
                    onSelectFile(file);
                    onTabChange('diff');
                  }}
                />
              </div>
            </div>
          )}

          {activeTab === 'diagram' && (
            <div className="bg-bg-secondary/20 flex min-h-0 w-full flex-1 flex-col">
              {task.diagram ? (
                <Mermaid chart={task.diagram} className="h-full w-full" />
              ) : (
                <div className="text-text-disabled flex h-full flex-col items-center justify-center gap-3">
                  <div className="bg-bg-secondary flex h-16 w-16 items-center justify-center rounded-2xl">
                    <ICONS.ICON_EMPTY size={32} weight="duotone" />
                  </div>
                  <p className="text-sm font-medium">No diagram available</p>
                </div>
              )}
            </div>
          )}
        </div>
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
    aria-pressed={active}
    className={`flex min-h-9 items-center gap-2 rounded-md px-3 text-xs font-medium transition-colors ${active ? 'bg-bg-tertiary text-text-primary' : 'text-text-tertiary hover:text-text-primary'}`}
  >
    <Icon size={14} />
    {label}
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

const TaskFiles: React.FC<{
  files: string[];
  availableFiles: DiffFile[];
  onSelect: (file: DiffFile) => void;
}> = ({ files, availableFiles, onSelect }) => (
  <div className="border-border mt-8 border-t pt-5">
    <h3 className="text-text-tertiary mb-3 text-xs font-medium">Related files</h3>
    <div className="flex flex-col gap-2">
      {files.map(path => {
        const file = availableFiles.find(file => file.name === path || file.new_path === path);
        return file ? (
          <button
            key={path}
            onClick={() => onSelect(file)}
            className="text-text-secondary hover:bg-bg-secondary flex items-center gap-2 rounded-md px-2 py-2 text-left font-mono text-xs break-all"
          >
            <ICONS.ICON_FILES size={15} className="shrink-0" />
            {path}
            <ICONS.CHEVRON_DOWN size={12} className="ml-auto shrink-0" />
          </button>
        ) : (
          <span key={path} className="text-text-tertiary font-mono text-xs break-all">
            {path}
          </span>
        );
      })}
    </div>
  </div>
);
