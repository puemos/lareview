import React, { useState, useEffect, useMemo } from 'react';
import { Brain, Trash, PencilSimple, Play, Check, X } from '@phosphor-icons/react';
import { ICONS } from '../../constants/icons';
import { PageHeader } from '../Common/PageHeader';
import { displayDate } from '../../utils/dates';
import { SearchField } from '../Common/SearchField';
import { EmptyPanel } from '../Common/EmptyPanel';
import { Dialog } from '../Common/Dialog';
import { InlineError } from '../Common/InlineError';
import { Toggle } from '../Common/Toggle';
import { Card } from '../ui/Card';
import { useLearnedPatterns } from '../../hooks/useLearnedPatterns';
import { useAgents } from '../../hooks/useAgents';
import type { LearnedPattern, LearnedPatternInput } from '../../types';

interface PatternDraft {
  pattern_text: string;
  category: string;
  file_extension: string;
  enabled: boolean;
}

const emptyDraft: PatternDraft = {
  pattern_text: '',
  category: '',
  file_extension: '',
  enabled: true,
};

const CATEGORY_OPTIONS = [
  'testing',
  'performance',
  'style',
  'error-handling',
  'security',
  'documentation',
  'naming',
  'other',
];

export const LearningView: React.FC = () => {
  const { patterns, status, create, update, remove, toggle, compact } = useLearnedPatterns();
  const { data: agents = [], isLoading: isLoadingAgents } = useAgents();

  const [patternToDelete, setPatternToDelete] = useState<LearnedPattern | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [patternFilter, setPatternFilter] = useState<'all' | 'enabled' | 'disabled'>('all');
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isCompactModalOpen, setIsCompactModalOpen] = useState(false);
  const [selectedAgentId, setSelectedAgentId] = useState<string>('');
  const [draft, setDraft] = useState<PatternDraft>(emptyDraft);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingDraft, setEditingDraft] = useState<PatternDraft>(emptyDraft);

  const patternList = patterns.data ?? [];
  const learningStatus = status.data;
  const visiblePatterns = patternList.filter(
    pattern =>
      (patternFilter === 'all' || pattern.enabled === (patternFilter === 'enabled')) &&
      [pattern.pattern_text, pattern.category ?? '', pattern.file_extension ?? ''].some(value =>
        value.toLowerCase().includes(searchQuery.trim().toLowerCase())
      )
  );

  // Select first available agent by default
  const availableAgents = useMemo(() => agents.filter(agent => agent.available), [agents]);
  useEffect(() => {
    if (!availableAgents.some(agent => agent.id === selectedAgentId)) {
      setSelectedAgentId(availableAgents[0]?.id ?? '');
    }
  }, [availableAgents, selectedAgentId]);

  const canSubmit = (state: PatternDraft) => {
    return state.pattern_text.trim().length > 0;
  };

  const toInput = (state: PatternDraft): LearnedPatternInput => ({
    pattern_text: state.pattern_text.trim(),
    category: state.category.trim() || null,
    file_extension: state.file_extension.trim() || null,
    enabled: state.enabled,
  });

  const handleCreate = () => {
    if (!canSubmit(draft)) return;
    create.mutate(toInput(draft), {
      onSuccess: () => {
        setDraft(emptyDraft);
        setIsAddModalOpen(false);
      },
    });
  };

  const startEdit = (pattern: LearnedPattern) => {
    setEditingId(pattern.id);
    setEditingDraft({
      pattern_text: pattern.pattern_text,
      category: pattern.category || '',
      file_extension: pattern.file_extension || '',
      enabled: pattern.enabled,
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditingDraft(emptyDraft);
  };

  const handleUpdate = () => {
    if (!editingId || !canSubmit(editingDraft)) return;
    update.mutate({ id: editingId, input: toInput(editingDraft) }, { onSuccess: cancelEdit });
  };

  const handleToggle = (pattern: LearnedPattern) => {
    toggle.mutate({ id: pattern.id, enabled: !pattern.enabled });
  };

  const openCompactModal = () => {
    setIsCompactModalOpen(true);
  };

  const handleRunCompaction = () => {
    if (
      !availableAgents.some(agent => agent.id === selectedAgentId) ||
      !learningStatus?.pending_rejections ||
      compact.isPending
    )
      return;
    compact.mutate(selectedAgentId, {
      onSuccess: () => {
        setIsCompactModalOpen(false);
      },
    });
  };

  const openAddModal = () => {
    setDraft(emptyDraft);
    setIsAddModalOpen(true);
  };

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <PageHeader
        title="Learning"
        description="Teach your agents which feedback is useful."
        actions={
          <>
            <button
              type="button"
              onClick={openCompactModal}
              disabled={compact.isPending}
              className="bg-bg-secondary text-text-secondary hover:text-text-primary border-border flex h-9 items-center gap-2 rounded-md border px-3 text-xs disabled:opacity-50"
            >
              <Play size={13} /> Analyze rejections
            </button>
            <button
              type="button"
              onClick={openAddModal}
              className="bg-brand text-brand-fg hover:bg-brand/90 flex h-9 items-center gap-2 rounded-md px-3 text-xs font-medium transition-colors"
            >
              <ICONS.ICON_PLUS size={14} /> New pattern
            </button>
          </>
        }
      />

      <div className="product-page-body">
        <div className="product-page-content space-y-6">
          {(patterns.isError || status.isError) && (
            <InlineError
              title={
                patterns.isError ? 'Could not load patterns' : 'Could not load learning status'
              }
              message={String(patterns.error ?? status.error)}
              onRetry={() => {
                void patterns.refetch();
                void status.refetch();
              }}
              isRetrying={patterns.isFetching || status.isFetching}
            />
          )}
          <div className="border-border/50 flex flex-wrap items-start justify-between gap-4 border-b pb-5">
            <p className="text-text-tertiary max-w-[65ch] text-[13px] leading-6">
              Ignore unhelpful feedback during a review, then analyze those decisions to shape
              future reviews. You can also add patterns yourself.
            </p>
            {learningStatus && (
              <div
                aria-label="Learning status"
                className="text-text-secondary shrink-0 space-y-1 text-xs leading-5"
              >
                <p>
                  {learningStatus.enabled_pattern_count} enabled · {learningStatus.pattern_count}{' '}
                  {learningStatus.pattern_count === 1 ? 'pattern' : 'patterns'}
                </p>
                <p className="text-text-disabled">
                  {learningStatus.pending_rejections} pending{' '}
                  {learningStatus.pending_rejections === 1 ? 'rejection' : 'rejections'}
                </p>
              </div>
            )}
          </div>

          {compact.isSuccess && compact.data && <CompactionResultBanner result={compact.data} />}

          {learningStatus && learningStatus.last_compaction_at && (
            <div className="text-text-tertiary text-[10px]">
              Last analysis: {displayDate(learningStatus.last_compaction_at) ?? 'Not recorded'}
            </div>
          )}

          {patternList.length > 0 && (
            <div className="flex flex-wrap items-center justify-between gap-3">
              <SearchField
                label="Search patterns"
                value={searchQuery}
                onChange={setSearchQuery}
                placeholder="Pattern text, category, or file extension"
              />
              <div
                role="group"
                aria-label="Filter patterns"
                className="bg-bg-secondary border-border flex h-9 items-center gap-1 rounded-md border p-0.5"
              >
                {(['all', 'enabled', 'disabled'] as const).map(filter => (
                  <button
                    key={filter}
                    type="button"
                    aria-pressed={patternFilter === filter}
                    onClick={() => setPatternFilter(filter)}
                    className={`h-8 rounded px-3 text-xs ${patternFilter === filter ? 'bg-bg-tertiary text-text-primary' : 'text-text-tertiary'}`}
                  >
                    {filter[0].toUpperCase() + filter.slice(1)} (
                    {
                      patternList.filter(
                        pattern => filter === 'all' || pattern.enabled === (filter === 'enabled')
                      ).length
                    }
                    )
                  </button>
                ))}
              </div>
            </div>
          )}
          {!patterns.isError &&
            (patternList.length > 0 && visiblePatterns.length === 0 ? (
              <EmptyPanel
                title="No matching patterns"
                description="Try another search or show all patterns."
                action={
                  <button
                    type="button"
                    onClick={() => {
                      setSearchQuery('');
                      setPatternFilter('all');
                    }}
                    className="text-text-secondary border-border rounded-md border px-3 py-2 text-xs"
                  >
                    Clear filters
                  </button>
                }
              />
            ) : (
              <PatternList
                patterns={visiblePatterns}
                editingId={editingId}
                editingDraft={editingDraft}
                onEdit={startEdit}
                onCancelEdit={cancelEdit}
                onEditChange={setEditingDraft}
                onUpdate={handleUpdate}
                onToggle={handleToggle}
                onDelete={id => {
                  remove.reset();
                  setPatternToDelete(patternList.find(pattern => pattern.id === id) ?? null);
                }}
                isLoading={patterns.isLoading}
                onAdd={openAddModal}
                isUpdating={update.isPending || remove.isPending || toggle.isPending}
                editError={update.error ? String(update.error) : undefined}
              />
            ))}
        </div>
      </div>

      <Dialog
        isOpen={Boolean(patternToDelete)}
        onClose={() => setPatternToDelete(null)}
        title="Delete pattern"
        busy={remove.isPending}
      >
        <p className="text-text-secondary text-sm leading-6">
          Remove this guidance from future reviews?
        </p>
        <blockquote className="text-text-primary border-border mt-4 border-l-2 pl-4 text-sm leading-6">
          {patternToDelete?.pattern_text}
        </blockquote>
        {remove.error && (
          <div className="mt-4">
            <InlineError title="Could not delete pattern" message={String(remove.error)} />
          </div>
        )}
        <div className="mt-6 flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setPatternToDelete(null)}
            disabled={remove.isPending}
            className="text-text-secondary rounded-md px-3 py-2 text-xs"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={remove.isPending}
            onClick={() => {
              if (patternToDelete)
                remove.mutate(patternToDelete.id, { onSuccess: () => setPatternToDelete(null) });
            }}
            className="bg-impact-blocking text-brand-fg rounded-md px-3 py-2 text-xs font-medium"
          >
            {remove.isPending ? 'Deleting…' : 'Delete pattern'}
          </button>
        </div>
      </Dialog>

      <AddPatternModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        draft={draft}
        onChange={setDraft}
        onSubmit={handleCreate}
        isSubmitting={create.isPending}
        canSubmit={canSubmit(draft)}
        error={create.error ? String(create.error) : undefined}
      />

      <AnalyzeRejectionsModal
        isOpen={isCompactModalOpen}
        onClose={() => !compact.isPending && setIsCompactModalOpen(false)}
        agents={availableAgents}
        selectedAgentId={selectedAgentId}
        onSelectAgent={setSelectedAgentId}
        onRun={handleRunCompaction}
        isRunning={compact.isPending}
        isLoadingAgents={isLoadingAgents}
        pendingCount={learningStatus?.pending_rejections ?? 0}
        error={compact.error ? String(compact.error) : undefined}
      />
    </div>
  );
};

interface CompactionResultBannerProps {
  result: {
    rejections_processed: number;
    patterns_created: number;
    patterns_updated: number;
    errors: string[];
  };
}

const CompactionResultBanner: React.FC<CompactionResultBannerProps> = ({ result }) => {
  const hasErrors = result.errors.length > 0;

  return (
    <div
      role={hasErrors ? 'alert' : 'status'}
      className={`rounded-lg border px-4 py-4 text-xs ${
        hasErrors
          ? 'bg-status-ignored/5 border-status-ignored/20'
          : 'border-emerald-500/20 bg-emerald-500/5'
      }`}
    >
      <div className="flex items-center gap-2 font-semibold">
        {hasErrors ? (
          <X size={14} className="text-status-ignored" />
        ) : (
          <Check size={14} className="text-emerald-400" />
        )}
        <span className={hasErrors ? 'text-status-ignored' : 'text-emerald-400'}>
          {hasErrors ? 'Analysis finished with issues' : 'Analysis complete'}
        </span>
      </div>
      <div className="text-text-secondary mt-1 ml-5 leading-relaxed">
        Processed {result.rejections_processed} rejections.
        <span className="mx-1">·</span>
        <span className="text-text-primary">{result.patterns_created} Created</span>
        <span className="mx-1">·</span>
        <span className="text-text-primary">{result.patterns_updated} Updated</span>
      </div>
      {hasErrors && (
        <div className="text-status-ignored mt-2 ml-5 space-y-1">
          {result.errors.map((err, i) => (
            <div key={i} className="flex items-center gap-1.5">
              <span className="bg-status-ignored h-1 w-1 rounded-full" />
              {err}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

interface AddPatternModalProps {
  isOpen: boolean;
  onClose: () => void;
  draft: PatternDraft;
  onChange: (draft: PatternDraft) => void;
  onSubmit: () => void;
  isSubmitting: boolean;
  canSubmit: boolean;
  error?: string;
}

const AddPatternModal: React.FC<AddPatternModalProps> = ({
  isOpen,
  onClose,
  draft,
  onChange,
  onSubmit,
  isSubmitting,
  canSubmit,
  error,
}) => {
  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="New pattern"
      description="Guide what your agents should prioritize or avoid."
      busy={isSubmitting}
    >
      {error && (
        <div className="mb-4">
          <InlineError title="Could not save pattern" message={error} />
        </div>
      )}
      <PatternForm
        draft={draft}
        onChange={onChange}
        onSubmit={onSubmit}
        submitLabel="Add pattern"
        disabled={!canSubmit}
        isLoading={isSubmitting}
        onCancel={onClose}
      />
    </Dialog>
  );
};

interface AnalyzeRejectionsModalProps {
  isOpen: boolean;
  onClose: () => void;
  agents: Array<{ id: string; name: string; description: string }>;
  selectedAgentId: string;
  onSelectAgent: (id: string) => void;
  onRun: () => void;
  isRunning: boolean;
  isLoadingAgents: boolean;
  pendingCount: number;
  error?: string;
}

const AnalyzeRejectionsModal: React.FC<AnalyzeRejectionsModalProps> = ({
  isOpen,
  onClose,
  agents,
  selectedAgentId,
  onSelectAgent,
  onRun,
  isRunning,
  isLoadingAgents,
  pendingCount,
  error,
}) => {
  const agentId = React.useId();
  const canRun =
    pendingCount > 0 &&
    agents.some(agent => agent.id === selectedAgentId) &&
    !isLoadingAgents &&
    !isRunning;
  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Analyze ignored feedback"
      description="Find recurring preferences and turn them into review guidance."
      busy={isRunning}
    >
      <div className="space-y-5">
        <p className="text-text-secondary text-sm leading-6">
          {pendingCount === 0
            ? 'No ignored feedback is waiting for analysis. Ignore unhelpful feedback during a review to give your agent examples to learn from.'
            : `${pendingCount} ignored feedback ${pendingCount === 1 ? 'item is' : 'items are'} ready for analysis.`}
        </p>
        <div className="product-form">
          <label htmlFor={agentId} className="text-text-secondary mb-2 block text-xs font-medium">
            Agent
          </label>
          {isLoadingAgents ? (
            <p role="status" className="text-text-tertiary text-xs">
              Loading agents…
            </p>
          ) : agents.length === 0 ? (
            <p className="text-text-tertiary text-xs leading-5">
              No agent is available. Configure an agent in Settings.
            </p>
          ) : (
            <select
              id={agentId}
              value={selectedAgentId}
              onChange={event => onSelectAgent(event.target.value)}
              disabled={isRunning}
              className="text-text-primary border-border w-full rounded-md border"
            >
              {agents.map(agent => (
                <option key={agent.id} value={agent.id}>
                  {agent.name}
                </option>
              ))}
            </select>
          )}
        </div>
        {error && <InlineError title="Analysis failed" message={error} />}
        {isRunning && (
          <p role="status" className="text-text-secondary flex items-center gap-2 text-xs">
            <ICONS.ACTION_LOADING size={14} className="animate-spin" /> Analyzing ignored feedback…
          </p>
        )}
        <div className="flex justify-end gap-3">
          <button
            type="button"
            onClick={onClose}
            disabled={isRunning}
            className="text-text-secondary rounded-md px-3 py-2 text-xs"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={onRun}
            disabled={!canRun}
            aria-busy={isRunning}
            className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled flex h-9 items-center gap-2 rounded-md px-4 text-xs font-medium"
          >
            <Play size={13} />
            {isRunning ? 'Analyzing…' : 'Analyze'}
          </button>
        </div>
      </div>
    </Dialog>
  );
};

interface PatternFormProps {
  draft: PatternDraft;
  onChange: (draft: PatternDraft) => void;
  onSubmit: () => void;
  submitLabel: string;
  disabled?: boolean;
  isLoading?: boolean;
  onCancel?: () => void;
  autoFocus?: boolean;
}

const PatternForm: React.FC<PatternFormProps> = ({
  draft,
  onChange,
  onSubmit,
  submitLabel,
  disabled,
  isLoading,
  onCancel,
  autoFocus = false,
}) => {
  const helpId = React.useId();
  return (
    <form
      className="product-form"
      onSubmit={event => {
        event.preventDefault();
        if (draft.pattern_text.trim() && !disabled && !isLoading) onSubmit();
      }}
    >
      <fieldset disabled={isLoading} className="min-w-0 space-y-4">
        <label className="block">
          <span>Pattern</span>
          <textarea
            value={draft.pattern_text}
            onChange={event => onChange({ ...draft, pattern_text: event.target.value })}
            required
            rows={4}
            autoFocus={autoFocus}
            data-autofocus
            aria-describedby={helpId}
            placeholder="Describe the feedback your agent should avoid or prioritize."
            className="text-text-primary placeholder:text-text-disabled border-border w-full rounded-md border"
          />
        </label>
        <div className="grid gap-4 md:grid-cols-2">
          <label className="block">
            <span>Category (optional)</span>
            <select
              value={draft.category}
              onChange={event => onChange({ ...draft, category: event.target.value })}
              className="text-text-primary border-border w-full rounded-md border"
            >
              <option value="">Any category</option>
              {CATEGORY_OPTIONS.map(category => (
                <option key={category} value={category}>
                  {category[0].toUpperCase() + category.slice(1).replace('-', ' ')}
                </option>
              ))}
            </select>
          </label>
          <label className="block">
            <span>File extension (optional)</span>
            <input
              value={draft.file_extension}
              onChange={event => onChange({ ...draft, file_extension: event.target.value })}
              placeholder="e.g. ts"
              className="text-text-primary placeholder:text-text-disabled border-border w-full rounded-md border font-mono"
            />
          </label>
        </div>
        <p id={helpId} className="text-text-tertiary text-xs leading-5">
          Make the pattern specific. Leave category and extension blank to apply it broadly.
        </p>
        <label className="text-text-secondary flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={event => onChange({ ...draft, enabled: event.target.checked })}
          />{' '}
          Use this pattern in new reviews
        </label>
        <div className="flex justify-end gap-3">
          {onCancel && (
            <button
              type="button"
              onClick={onCancel}
              className="text-text-secondary rounded-md px-3 py-2 text-xs"
            >
              Cancel
            </button>
          )}
          <button
            type="submit"
            disabled={disabled || isLoading || !draft.pattern_text.trim()}
            className="bg-brand text-brand-fg disabled:bg-bg-tertiary disabled:text-text-disabled flex items-center gap-2 rounded-md px-4 py-2 text-xs font-medium"
          >
            {isLoading && <ICONS.ACTION_LOADING size={13} className="animate-spin" />}
            {isLoading ? 'Saving…' : submitLabel}
          </button>
        </div>
      </fieldset>
    </form>
  );
};

interface PatternListProps {
  patterns: LearnedPattern[];
  editingId: string | null;
  editingDraft: PatternDraft;
  onEdit: (pattern: LearnedPattern) => void;
  onCancelEdit: () => void;
  onEditChange: (draft: PatternDraft) => void;
  onUpdate: () => void;
  onToggle: (pattern: LearnedPattern) => void;
  onDelete: (id: string) => void;
  isLoading: boolean;
  onAdd: () => void;
  isUpdating: boolean;
  editError?: string;
}

const PatternList: React.FC<PatternListProps> = ({
  patterns,
  editingId,
  editingDraft,
  onEdit,
  onCancelEdit,
  onEditChange,
  onUpdate,
  onToggle,
  onDelete,
  onAdd,
  isLoading,
  isUpdating,
  editError,
}) => {
  if (isLoading) {
    return (
      <div role="status" aria-label="Loading patterns" className="animate-pulse space-y-3">
        {[1, 2].map(index => (
          <div
            key={index}
            aria-hidden="true"
            className="bg-bg-secondary border-border/50 h-32 rounded-lg border"
          />
        ))}
      </div>
    );
  }
  if (patterns.length === 0) {
    return (
      <EmptyPanel
        icon={<Brain size={28} />}
        title="No learned patterns yet"
        description="Add a pattern yourself, or ignore unhelpful feedback during reviews and analyze those decisions."
        action={
          <button
            type="button"
            onClick={onAdd}
            className="text-text-secondary border-border rounded-md border px-3 py-2 text-xs"
          >
            Add a pattern
          </button>
        }
      />
    );
  }

  return (
    <div className="space-y-3">
      {patterns.map(pattern => (
        <Card key={pattern.id} className="overflow-hidden">
          {editingId === pattern.id ? (
            <div className="p-4">
              {editError && (
                <div className="mb-4">
                  <InlineError title="Could not update pattern" message={editError} />
                </div>
              )}
              <PatternForm
                draft={editingDraft}
                onChange={onEditChange}
                onSubmit={onUpdate}
                submitLabel="Save Changes"
                onCancel={onCancelEdit}
                autoFocus
                disabled={!editingDraft.pattern_text.trim()}
                isLoading={isUpdating}
              />
            </div>
          ) : (
            <div className="flex flex-col">
              {/* Top: Description and Toggle */}
              <div className="space-y-3 p-4">
                <div className="flex items-start justify-between gap-4">
                  <div className="text-text-primary flex-1 text-sm leading-6">
                    {pattern.pattern_text}
                  </div>
                  <Toggle
                    label={`Pattern: ${pattern.pattern_text}`}
                    checked={pattern.enabled}
                    onChange={() => onToggle(pattern)}
                    disabled={isUpdating}
                  />
                </div>

                {/* Middle: Tags */}
                <div className="text-text-tertiary flex flex-wrap items-center gap-2 text-[10px]">
                  {pattern.category && (
                    <span className="text-text-tertiary border-border/50 rounded-md border px-2 py-0.5 text-[11px]">
                      {pattern.category.replace('-', ' ')}
                    </span>
                  )}
                  {pattern.file_extension && (
                    <span className="text-text-tertiary border-border/50 rounded-md border px-2 py-0.5 font-mono text-[11px]">
                      *.{pattern.file_extension}
                    </span>
                  )}
                  {pattern.source_count > 0 && (
                    <span className="bg-bg-tertiary text-text-secondary border-border/50 rounded-md border px-2 py-0.5">
                      {pattern.source_count} Rejection{pattern.source_count !== 1 ? 's' : ''}
                    </span>
                  )}
                </div>
              </div>

              {/* Bottom: Footer with Actions */}
              <div className="border-border/50 flex items-center justify-between border-t px-4 py-3">
                <div className="text-text-tertiary flex items-center gap-4 text-[10px]">
                  {displayDate(pattern.updated_at) && (
                    <time dateTime={pattern.updated_at}>
                      Updated {displayDate(pattern.updated_at)}
                    </time>
                  )}
                </div>

                <div className="flex items-center gap-2">
                  <button
                    onClick={() => onEdit(pattern)}
                    disabled={isUpdating}
                    className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary flex h-8 items-center gap-1.5 rounded-md px-2 text-xs transition-colors"
                    aria-label={`Edit pattern: ${pattern.pattern_text}`}
                  >
                    <PencilSimple size={13} /> Edit
                  </button>
                  <button
                    onClick={() => onDelete(pattern.id)}
                    disabled={isUpdating}
                    className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 rounded-md border border-transparent p-1.5 transition-colors"
                    title="Delete pattern"
                    aria-label={`Delete pattern: ${pattern.pattern_text}`}
                  >
                    <Trash size={14} />
                  </button>
                </div>
              </div>
            </div>
          )}
        </Card>
      ))}
    </div>
  );
};
