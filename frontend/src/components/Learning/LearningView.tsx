import React, { useState, useEffect } from 'react';
import { Brain, Trash, PencilSimple, Play, Lightning, Check, X } from '@phosphor-icons/react';
import { ICONS } from '../../constants/icons';
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

  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isCompactModalOpen, setIsCompactModalOpen] = useState(false);
  const [selectedAgentId, setSelectedAgentId] = useState<string>('');
  const [draft, setDraft] = useState<PatternDraft>(emptyDraft);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingDraft, setEditingDraft] = useState<PatternDraft>(emptyDraft);

  const patternList = patterns.data ?? [];
  const learningStatus = status.data;

  // Select first available agent by default
  const availableAgents = agents.filter(a => a.available);
  useEffect(() => {
    if (availableAgents.length > 0 && !selectedAgentId) {
      setSelectedAgentId(availableAgents[0].id);
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
    update.mutate(
      { id: editingId, input: toInput(editingDraft) },
      { onSuccess: cancelEdit }
    );
  };

  const handleToggle = (pattern: LearnedPattern) => {
    toggle.mutate({ id: pattern.id, enabled: !pattern.enabled });
  };

  const openCompactModal = () => {
    setIsCompactModalOpen(true);
  };

  const handleRunCompaction = () => {
    if (!selectedAgentId) return;
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
      <div className="border-border bg-bg-primary flex h-12 shrink-0 items-center justify-between border-b px-6">
        <div className="flex items-center gap-3">
          <Brain size={18} weight="fill" className="text-brand" />
          <h1 className="font-display text-text-primary text-sm font-medium tracking-wide">
            Learning
          </h1>
          {learningStatus && (
            <div className="ml-2 flex items-center gap-1.5">
              <span className="bg-bg-tertiary text-text-secondary rounded-full px-2 py-0.5 text-[9px] font-medium">
                {learningStatus.enabled_pattern_count} Patterns
              </span>
              {learningStatus.pending_rejections > 0 && (
                <span className="bg-status-in_progress/10 text-status-in_progress rounded-full px-2 py-0.5 text-[9px] font-medium">
                  {learningStatus.pending_rejections} Pending
                </span>
              )}
            </div>
          )}
        </div>
        <div className="flex items-center gap-1.5">
          <button
            onClick={openCompactModal}
            disabled={(learningStatus?.pending_rejections ?? 0) === 0}
            className="bg-bg-tertiary text-text-secondary hover:text-text-primary border-border flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-[10px] font-bold transition-all hover:brightness-110 disabled:opacity-50 disabled:cursor-not-allowed"
          >
            <Play size={12} weight="fill" />
            Analyze Rejections
          </button>
          <button
            onClick={openAddModal}
            className="bg-brand text-bg-primary shadow-custom flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[10px] font-bold transition-all hover:brightness-110 active:scale-95"
          >
            <ICONS.ICON_PLUS size={12} weight="bold" />
            New Pattern
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-5xl space-y-8">
          <div className="text-text-tertiary max-w-2xl text-[11px] leading-relaxed">
            Learned patterns help the AI avoid generating unhelpful feedback. Mark feedback
            as <span className="text-text-primary font-medium">&quot;ignored&quot;</span> during reviews to record rejections.
            Analyzing them identifies patterns that calibrate future reviews.
          </div>

          {compact.isSuccess && compact.data && (
            <CompactionResultBanner result={compact.data} />
          )}

          {learningStatus && learningStatus.last_compaction_at && (
            <div className="text-text-tertiary text-[10px]">
              Last analysis: {new Date(learningStatus.last_compaction_at).toLocaleString()}
            </div>
          )}

          <PatternList
            patterns={patternList}
            editingId={editingId}
            editingDraft={editingDraft}
            onEdit={startEdit}
            onCancelEdit={cancelEdit}
            onEditChange={setEditingDraft}
            onUpdate={handleUpdate}
            onToggle={handleToggle}
            onDelete={(id) => remove.mutate(id)}
            isLoading={patterns.isLoading}
          />
        </div>
      </div>

      <AddPatternModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        draft={draft}
        onChange={setDraft}
        onSubmit={handleCreate}
        isSubmitting={create.isPending}
        canSubmit={canSubmit(draft)}
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
      className={`rounded-xl px-4 py-3 text-[11px] border shadow-sm animate-in fade-in slide-in-from-top-2 duration-300 ${
        hasErrors
          ? 'bg-status-ignored/5 border-status-ignored/20'
          : 'bg-emerald-500/5 border-emerald-500/20'
      }`}
    >
      <div className="flex items-center gap-2 font-semibold">
        {hasErrors ? (
          <X size={14} className="text-status-ignored" />
        ) : (
          <Check size={14} className="text-emerald-400" />
        )}
        <span className={hasErrors ? 'text-status-ignored' : 'text-emerald-400'}>
          Analysis Complete
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
              <span className="h-1 w-1 rounded-full bg-status-ignored" />
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
}

const AddPatternModal: React.FC<AddPatternModalProps> = ({
  isOpen,
  onClose,
  draft,
  onChange,
  onSubmit,
  isSubmitting,
  canSubmit,
}) => {
  if (!isOpen) return null;

  return (
    <div className="animate-in fade-in fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-[2px] duration-200">
      <div className="bg-bg-primary border-border animate-in zoom-in-95 flex max-h-[85vh] w-full max-w-lg flex-col rounded-xl border shadow-2xl duration-200">
        <div className="flex items-center justify-between px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-brand/10 text-brand rounded-md p-1.5">
              <ICONS.ICON_PLUS size={16} weight="bold" />
            </div>
            <h3 className="text-text-primary text-[13px] font-semibold">New Pattern</h3>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded-md p-1 transition-all"
          >
            <ICONS.ACTION_CLOSE size={16} />
          </button>
        </div>
        <div className="custom-scrollbar flex-1 overflow-y-auto px-6 pb-6">
          <PatternForm
            draft={draft}
            onChange={onChange}
            onSubmit={onSubmit}
            submitLabel="Add Pattern"
            disabled={!canSubmit}
            isLoading={isSubmitting}
            onCancel={onClose}
          />
        </div>
      </div>
    </div>
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
  if (!isOpen) return null;

  return (
    <div className="animate-in fade-in fixed inset-0 z-50 flex items-center justify-center bg-black/40 p-4 backdrop-blur-[2px] duration-200">
      <div className="bg-bg-primary border-border animate-in zoom-in-95 w-full max-w-sm rounded-xl border shadow-2xl duration-200">
        <div className="flex items-center justify-between px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-brand/10 text-brand rounded-md p-1.5">
              <Brain size={16} weight="fill" />
            </div>
            <h3 className="text-text-primary text-[13px] font-semibold">Analyze Rejections</h3>
          </div>
          <button
            onClick={onClose}
            disabled={isRunning}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded-md p-1 transition-all disabled:opacity-50"
          >
            <ICONS.ACTION_CLOSE size={16} />
          </button>
        </div>

        <div className="px-6 pb-6 space-y-5">
          <div className="text-text-secondary text-[11px] leading-relaxed">
            The AI will analyze <span className="text-brand font-bold">{pendingCount} pending rejections</span> to
            generate new patterns and calibrate future reviews.
          </div>

          <div className="space-y-2">
            <label className="text-text-disabled text-[9px] font-bold tracking-wider uppercase">
              Select AI Agent
            </label>
            {isLoadingAgents ? (
              <div className="bg-bg-tertiary/50 border-border text-text-tertiary rounded-md border px-3 py-2 text-[11px]">
                Loading agents...
              </div>
            ) : agents.length === 0 ? (
              <div className="bg-bg-tertiary/50 border-border text-status-ignored rounded-md border px-3 py-2 text-[11px]">
                No available agents.
              </div>
            ) : (
              <select
                value={selectedAgentId}
                onChange={(e) => onSelectAgent(e.target.value)}
                disabled={isRunning}
                className="bg-bg-tertiary/50 border-border text-text-primary focus:border-brand focus:ring-brand/10 w-full rounded-md border px-2.5 py-2 text-[11px] transition-all focus:ring-2 focus:outline-none cursor-pointer"
              >
                {agents.map((agent) => (
                  <option key={agent.id} value={agent.id}>
                    {agent.name}
                  </option>
                ))}
              </select>
            )}
          </div>

          {error && (
            <div className="bg-status-ignored/10 border-status-ignored/20 text-status-ignored rounded-md border p-3 text-[10px] leading-relaxed">
              {error}
            </div>
          )}

          {isRunning && (
            <div className="bg-brand/5 border-brand/20 rounded-md border p-3 animate-pulse">
              <div className="flex items-center gap-2">
                <Lightning size={14} weight="fill" className="text-brand" />
                <span className="text-brand text-[11px] font-bold">In progress...</span>
              </div>
            </div>
          )}

          <div className="flex items-center gap-2 pt-1">
            <button
              onClick={onClose}
              disabled={isRunning}
              className="flex-1 bg-bg-tertiary text-text-secondary hover:text-text-primary rounded-md py-2 text-[11px] font-bold transition-all disabled:opacity-50"
            >
              Cancel
            </button>
            <button
              onClick={onRun}
              disabled={isRunning || !selectedAgentId || agents.length === 0}
              className="flex-[2] bg-brand text-bg-primary disabled:opacity-50 flex items-center justify-center gap-2 rounded-md py-2 text-[11px] font-bold transition-all hover:brightness-105"
            >
              {isRunning ? (
                <>
                  <ICONS.ACTION_LOADING size={12} className="animate-spin" />
                  Running...
                </>
              ) : (
                <>
                  <Play size={12} weight="fill" />
                  Analyze
                </>
              )}
            </button>
          </div>
        </div>
      </div>
    </div>
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
}

const PatternForm: React.FC<PatternFormProps> = ({
  draft,
  onChange,
  onSubmit,
  submitLabel,
  disabled,
  isLoading,
  onCancel,
}) => {
  return (
    <div className="space-y-4 py-2">
      <div className="space-y-1.5">
        <label className="text-text-disabled text-[9px] font-bold tracking-wider uppercase">
          Pattern Text
        </label>
        <textarea
          value={draft.pattern_text}
          onChange={(e) => onChange({ ...draft, pattern_text: e.target.value })}
          rows={2}
          placeholder="e.g., Don't flag unwrap() in test files"
          className="bg-bg-tertiary/50 border-border text-text-primary placeholder-text-tertiary focus:border-brand focus:ring-brand/10 w-full resize-none rounded-md border px-2.5 py-2 text-[11px] transition-all focus:ring-2 focus:outline-none"
        />
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        <div className="space-y-1.5">
          <label className="text-text-disabled text-[9px] font-bold tracking-wider uppercase">
            Category
          </label>
          <select
            value={draft.category}
            onChange={(e) => onChange({ ...draft, category: e.target.value })}
            className="bg-bg-tertiary/50 border-border text-text-primary focus:border-brand focus:ring-brand/10 w-full rounded-md border px-2 py-1.5 text-[11px] transition-all focus:ring-2 focus:outline-none cursor-pointer"
          >
            <option value="">None</option>
            {CATEGORY_OPTIONS.map((cat) => (
              <option key={cat} value={cat}>
                {cat.charAt(0).toUpperCase() + cat.slice(1).replace('-', ' ')}
              </option>
            ))}
          </select>
        </div>

        <div className="space-y-1.5">
          <label className="text-text-disabled text-[9px] font-bold tracking-wider uppercase">
            Filter by Extension
          </label>
          <input
            type="text"
            value={draft.file_extension}
            onChange={(e) => onChange({ ...draft, file_extension: e.target.value })}
            placeholder="e.g., rs, ts"
            className="bg-bg-tertiary/50 border-border text-text-primary placeholder-text-tertiary focus:border-brand focus:ring-brand/10 w-full rounded-md border px-2.5 py-1.5 text-[11px] transition-all focus:ring-2 focus:outline-none"
          />
        </div>
      </div>

      <div className="flex items-center justify-between pt-2">
        <label className="text-text-secondary flex items-center gap-2 text-[11px] cursor-pointer select-none">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={(e) => onChange({ ...draft, enabled: e.target.checked })}
            className="accent-brand h-3.5 w-3.5"
          />
          Enabled
        </label>

        <div className="flex items-center gap-2">
          {onCancel && (
            <button
              onClick={onCancel}
              className="text-text-tertiary hover:text-text-primary px-3 py-1.5 text-[11px] font-medium transition-all"
            >
              Cancel
            </button>
          )}
          <button
            onClick={onSubmit}
            disabled={disabled || isLoading}
            className="bg-brand text-bg-primary disabled:opacity-50 flex items-center gap-1.5 rounded-md px-4 py-1.5 text-[11px] font-bold transition-all hover:brightness-105 disabled:cursor-not-allowed"
          >
            {isLoading && <ICONS.ACTION_LOADING size={10} className="animate-spin" />}
            {submitLabel}
          </button>
        </div>
      </div>
    </div>
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
  isLoading,
}) => {
  if (isLoading) {
    return (
      <div className="flex items-center gap-3 py-12 justify-center text-text-tertiary">
        <ICONS.ACTION_LOADING size={16} className="animate-spin text-brand" />
        <span className="text-[11px] font-medium tracking-wide uppercase opacity-70">Loading Patterns...</span>
      </div>
    );
  }

  if (patterns.length === 0) {
    return (
      <div className="bg-bg-secondary/20 border-border group flex flex-col items-center justify-center rounded-lg border border-dashed py-20 transition-colors hover:bg-bg-secondary/30">
        <div className="bg-bg-tertiary/50 mb-4 flex h-14 w-14 items-center justify-center rounded-full transition-transform group-hover:scale-110">
          <Brain size={28} weight="duotone" className="text-text-tertiary/60" />
        </div>
        <div className="text-text-primary mb-1 text-sm font-semibold">No learned patterns yet</div>
        <div className="text-text-tertiary max-w-xs text-center text-[11px] leading-relaxed opacity-80">
          Mark feedback as ignored during reviews to help the AI learn your preferences.
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-3">
      {patterns.map((pattern, idx) => (
        <Card
          key={pattern.id}
          className="overflow-hidden animate-in fade-in slide-in-from-top-1 fill-mode-both"
          style={{ animationDelay: `${idx * 40}ms` }}
        >
          {editingId === pattern.id ? (
            <div className="p-4">
              <PatternForm
                draft={editingDraft}
                onChange={onEditChange}
                onSubmit={onUpdate}
                submitLabel="Save Changes"
                onCancel={onCancelEdit}
              />
            </div>
          ) : (
            <div className="flex flex-col">
              {/* Top: Description and Toggle */}
              <div className="p-4 space-y-4">
                <div className="flex justify-between items-start gap-4">
                  <div className="text-text-primary text-sm font-medium leading-relaxed flex-1">
                    {pattern.pattern_text}
                  </div>
                  <button
                    onClick={() => onToggle(pattern)}
                    className={`shrink-0 rounded-md px-2.5 py-1 text-[10px] font-semibold transition-colors ${
                      pattern.enabled
                        ? 'bg-status-done/10 text-status-done border border-status-done/20'
                        : 'bg-bg-tertiary text-text-tertiary border border-border'
                    }`}
                  >
                    {pattern.enabled ? 'Enabled' : 'Disabled'}
                  </button>
                </div>

                {/* Middle: Tags */}
                <div className="text-text-tertiary flex flex-wrap items-center gap-2 text-[10px]">
                  {pattern.category && (
                    <span className="bg-bg-tertiary text-blue-400 border border-blue-500/20 rounded-md px-2 py-0.5 font-bold uppercase tracking-wider">
                      {pattern.category.replace('-', ' ')}
                    </span>
                  )}
                  {pattern.file_extension && (
                    <span className="bg-bg-tertiary text-emerald-400 border border-emerald-500/20 rounded-md px-2 py-0.5 font-mono font-bold uppercase tracking-wider">
                      *.{pattern.file_extension}
                    </span>
                  )}
                  {pattern.source_count > 0 && (
                    <span className="bg-bg-tertiary text-text-secondary border border-border/50 rounded-md px-2 py-0.5">
                      {pattern.source_count} Rejection{pattern.source_count !== 1 ? 's' : ''}
                    </span>
                  )}
                </div>
              </div>

              {/* Bottom: Footer with Actions */}
              <div className="border-t border-border/50 px-4 py-3 flex items-center justify-between">
                <div className="text-text-tertiary flex items-center gap-4 text-[10px]">
                  {/* Placeholder for future metadata if needed */}
                  <span className="opacity-0">.</span>
                </div>

                <div className="flex items-center gap-1.5 opacity-0 transition-opacity group-hover:opacity-100">
                  <button
                    onClick={() => onEdit(pattern)}
                    className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded-md border border-transparent p-1.5 transition-colors"
                    title="Edit"
                  >
                    <PencilSimple size={14} />
                  </button>
                  <button
                    onClick={() => onDelete(pattern.id)}
                    className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 rounded-md border border-transparent p-1.5 transition-colors"
                    title="Delete"
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
