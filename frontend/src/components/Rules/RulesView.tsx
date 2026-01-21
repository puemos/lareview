import React, { useMemo, useState } from 'react';
import { Asterisk, Check, PencilSimple, Trash, Warning, ChartBar } from '@phosphor-icons/react';
import { ICONS } from '../../constants/icons';
import { useRules, useRuleRejectionStats, type ReviewRuleInput, type RuleRejectionStats } from '../../hooks/useRules';
import { useRepos } from '../../hooks/useRepos';
import { RuleLibraryModal } from './RuleLibraryModal';
import type { LinkedRepo, ReviewRule, RuleScope } from '../../types';

interface RuleDraft {
  scope: RuleScope;
  repo_id: string;
  glob: string;
  category: string;
  text: string;
  enabled: boolean;
}

const emptyDraft: RuleDraft = {
  scope: 'global',
  repo_id: '',
  glob: '',
  category: '',
  text: '',
  enabled: true,
};

export const RulesView: React.FC = () => {
  const { data: rules = [], isLoading, createRule, updateRule, removeRule } = useRules();
  const { data: repos = [] } = useRepos();
  const { data: rejectionStats = [] } = useRuleRejectionStats();

  const [draft, setDraft] = useState<RuleDraft>(emptyDraft);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isLibraryOpen, setIsLibraryOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingDraft, setEditingDraft] = useState<RuleDraft>(emptyDraft);

  const globalRules = useMemo(() => rules.filter(rule => rule.scope === 'global'), [rules]);
  const repoRules = useMemo(() => rules.filter(rule => rule.scope === 'repo'), [rules]);

  // Create a map of rule_id to stats for quick lookup
  const statsById = useMemo(() => {
    const map: Record<string, RuleRejectionStats> = {};
    rejectionStats.forEach(stat => {
      map[stat.rule_id] = stat;
    });
    return map;
  }, [rejectionStats]);

  const repoName = (repoId?: string | null) =>
    repos.find(repo => repo.id === repoId)?.name || 'Unknown repo';

  const canSubmit = (state: RuleDraft) => {
    if (!state.text.trim()) return false;
    if (state.scope === 'repo' && !state.repo_id) return false;
    return true;
  };

  const toInput = (state: RuleDraft): ReviewRuleInput => ({
    scope: state.scope,
    repo_id: state.scope === 'repo' ? state.repo_id || null : null,
    glob: state.glob.trim() ? state.glob.trim() : null,
    category: state.category.trim() ? state.category.trim() : null,
    text: state.text.trim(),
    enabled: state.enabled,
  });

  const handleCreate = () => {
    if (!canSubmit(draft)) return;
    createRule.mutate(toInput(draft), {
      onSuccess: () => {
        setDraft(emptyDraft);
        setIsAddModalOpen(false);
      },
    });
  };

  const startEdit = (rule: ReviewRule) => {
    setEditingId(rule.id);
    setEditingDraft({
      scope: rule.scope,
      repo_id: rule.repo_id || '',
      glob: rule.glob || '',
      category: rule.category || '',
      text: rule.text,
      enabled: rule.enabled,
    });
  };

  const cancelEdit = () => {
    setEditingId(null);
    setEditingDraft(emptyDraft);
  };

  const handleUpdate = () => {
    if (!editingId || !canSubmit(editingDraft)) return;
    updateRule.mutate({ id: editingId, input: toInput(editingDraft) }, { onSuccess: cancelEdit });
  };

  const toggleRule = (rule: ReviewRule) => {
    updateRule.mutate({
      id: rule.id,
      input: {
        scope: rule.scope,
        repo_id: rule.repo_id || null,
        glob: rule.glob || null,
        category: rule.category || null,
        text: rule.text,
        enabled: !rule.enabled,
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
          <Asterisk size={18} weight="fill" className="text-brand" />
          <h1 className="font-display text-text-primary text-sm font-medium tracking-wide">
            Rules
          </h1>
        </div>
        <div className="flex items-center gap-2">
          <button
            onClick={() => setIsLibraryOpen(true)}
            className="bg-bg-tertiary text-text-secondary hover:text-text-primary border-border flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-[10px] font-bold transition-all hover:brightness-110"
          >
            <ICONS.ICON_PLAN size={12} />
            Library
          </button>
          <button
            onClick={openAddModal}
            className="bg-brand text-bg-primary shadow-custom flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[10px] font-bold transition-all hover:brightness-110"
          >
            <ICONS.ICON_PLUS size={12} weight="bold" />
            Add Rule
          </button>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-5xl space-y-6">
          <div className="text-text-tertiary text-xs leading-relaxed">
            Rules define what the AI should verify during code review. Each rule is checked and
            findings are reported. Apply them globally or scope them to a repository and optional
            glob pattern.
          </div>

          <RuleSection
            title="Global Rules"
            rules={globalRules}
            repos={repos}
            statsById={statsById}
            editingId={editingId}
            editingDraft={editingDraft}
            onEdit={startEdit}
            onCancelEdit={cancelEdit}
            onEditChange={setEditingDraft}
            onUpdate={handleUpdate}
            onToggle={toggleRule}
            onDelete={removeRule.mutate}
            repoName={repoName}
            isLoading={isLoading}
          />
          <RuleSection
            title="Repository Rules"
            rules={repoRules}
            repos={repos}
            statsById={statsById}
            editingId={editingId}
            editingDraft={editingDraft}
            onEdit={startEdit}
            onCancelEdit={cancelEdit}
            onEditChange={setEditingDraft}
            onUpdate={handleUpdate}
            onToggle={toggleRule}
            onDelete={removeRule.mutate}
            repoName={repoName}
            isLoading={isLoading}
          />
        </div>
      </div>

      <AddRuleModal
        isOpen={isAddModalOpen}
        onClose={() => setIsAddModalOpen(false)}
        draft={draft}
        repos={repos}
        onChange={setDraft}
        onSubmit={handleCreate}
        isSubmitting={createRule.isPending}
        canSubmit={canSubmit(draft)}
      />

      <RuleLibraryModal
        isOpen={isLibraryOpen}
        onClose={() => setIsLibraryOpen(false)}
        repos={repos}
      />
    </div>
  );
};

interface AddRuleModalProps {
  isOpen: boolean;
  onClose: () => void;
  draft: RuleDraft;
  repos: LinkedRepo[];
  onChange: (draft: RuleDraft) => void;
  onSubmit: () => void;
  isSubmitting: boolean;
  canSubmit: boolean;
}

const AddRuleModal: React.FC<AddRuleModalProps> = ({
  isOpen,
  onClose,
  draft,
  repos,
  onChange,
  onSubmit,
  isSubmitting,
  canSubmit,
}) => {
  if (!isOpen) return null;

  return (
    <div className="animate-in fade-in fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm duration-200">
      <div className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex max-h-[85vh] w-full max-w-2xl flex-col rounded-xl border shadow-2xl duration-200">
        <div className="border-border/50 bg-bg-secondary/30 flex items-center justify-between rounded-t-xl border-b px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-accent/10 text-accent rounded-md p-1.5">
              <ICONS.ICON_PLUS size={18} />
            </div>
            <h3 className="text-text-primary text-sm font-semibold">Add Rule</h3>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded-md p-1 transition-all"
          >
            <ICONS.ACTION_CLOSE size={18} />
          </button>
        </div>
        <div className="custom-scrollbar flex-1 overflow-y-auto p-6">
          <RuleForm
            draft={draft}
            repos={repos}
            onChange={onChange}
            onSubmit={onSubmit}
            submitLabel="Add Rule"
            disabled={!canSubmit}
            isLoading={isSubmitting}
            onCancel={onClose}
          />
        </div>
      </div>
    </div>
  );
};

interface RuleFormProps {
  draft: RuleDraft;
  repos: LinkedRepo[];
  onChange: (draft: RuleDraft) => void;
  onSubmit: () => void;
  submitLabel: string;
  disabled?: boolean;
  isLoading?: boolean;
  onCancel?: () => void;
}

const RuleForm: React.FC<RuleFormProps> = ({
  draft,
  repos,
  onChange,
  onSubmit,
  submitLabel,
  disabled,
  isLoading,
  onCancel,
}) => {
  const repoRequired = draft.scope === 'repo' && repos.length === 0;

  return (
    <div className="space-y-4">
      <div className="grid gap-4 md:grid-cols-2">
        <label className="space-y-1">
          <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
            Scope
          </span>
          <select
            value={draft.scope}
            onChange={e => onChange({ ...draft, scope: e.target.value as RuleScope })}
            className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 w-full rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
          >
            <option value="global">Global</option>
            <option value="repo">Repository</option>
          </select>
        </label>

        <label className="space-y-1">
          <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
            Repository
          </span>
          <select
            value={draft.repo_id}
            onChange={e => onChange({ ...draft, repo_id: e.target.value })}
            disabled={draft.scope !== 'repo'}
            className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 w-full rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none disabled:opacity-50"
          >
            <option value="">Select a repository</option>
            {repos.map(repo => (
              <option key={repo.id} value={repo.id}>
                {repo.name}
              </option>
            ))}
          </select>
        </label>
      </div>

      {repoRequired && (
        <div className="text-text-tertiary text-xs">
          Link a repository to enable repo-scoped rules.
        </div>
      )}

      <div className="grid gap-4 md:grid-cols-2">
        <label className="space-y-1">
          <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
            Category (Optional)
          </span>
          <input
            type="text"
            value={draft.category}
            onChange={e => onChange({ ...draft, category: e.target.value })}
            placeholder="e.g., security, performance"
            className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 w-full rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
          />
        </label>

        <label className="space-y-1">
          <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
            Glob Pattern (Optional)
          </span>
          <input
            type="text"
            value={draft.glob}
            onChange={e => onChange({ ...draft, glob: e.target.value })}
            placeholder="src/**/*.rs"
            className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 w-full rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
          />
        </label>
      </div>

      <label className="space-y-1">
        <span className="text-text-disabled text-[10px] font-bold tracking-wider uppercase">
          Rule Text
        </span>
        <textarea
          value={draft.text}
          onChange={e => onChange({ ...draft, text: e.target.value })}
          rows={4}
          placeholder="Describe what the AI should check for and report on."
          className="bg-bg-tertiary border-border text-text-primary placeholder-text-disabled focus:border-brand focus:ring-brand/20 w-full resize-none rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
        />
      </label>

      <label className="text-text-secondary flex items-center gap-2 text-xs">
        <input
          type="checkbox"
          checked={draft.enabled}
          onChange={e => onChange({ ...draft, enabled: e.target.checked })}
          className="accent-brand"
        />
        Enabled
      </label>

      <div className="flex items-center gap-2">
        <button
          onClick={onSubmit}
          disabled={disabled || repoRequired || isLoading}
          className="bg-brand text-bg-primary disabled:bg-bg-tertiary disabled:text-text-disabled rounded-md px-3 py-2 text-xs font-semibold transition-all hover:brightness-110 disabled:cursor-not-allowed"
        >
          {submitLabel}
        </button>
        {onCancel && (
          <button
            onClick={onCancel}
            className="bg-bg-tertiary text-text-secondary hover:text-text-primary border-border rounded-md border px-3 py-2 text-xs font-semibold transition-all"
          >
            Cancel
          </button>
        )}
      </div>
    </div>
  );
};

interface RuleSectionProps {
  title: string;
  rules: ReviewRule[];
  repos: LinkedRepo[];
  statsById: Record<string, RuleRejectionStats>;
  editingId: string | null;
  editingDraft: RuleDraft;
  onEdit: (rule: ReviewRule) => void;
  onCancelEdit: () => void;
  onEditChange: (draft: RuleDraft) => void;
  onUpdate: () => void;
  onToggle: (rule: ReviewRule) => void;
  onDelete: (id: string) => void;
  repoName: (repoId?: string | null) => string;
  isLoading: boolean;
}

const RuleSection: React.FC<RuleSectionProps> = ({
  title,
  rules,
  repos,
  statsById,
  editingId,
  editingDraft,
  onEdit,
  onCancelEdit,
  onEditChange,
  onUpdate,
  onToggle,
  onDelete,
  repoName,
  isLoading,
}) => {
  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-text-primary text-sm font-semibold">{title}</h3>
        {isLoading && <span className="text-text-tertiary text-[10px]">Loading...</span>}
      </div>

      {rules.length === 0 ? (
        <div className="text-text-tertiary bg-bg-secondary/40 border-border rounded-lg border p-4 text-xs">
          No rules configured yet.
        </div>
      ) : (
        <div className="space-y-3">
          {rules.map(rule => (
            <div
              key={rule.id}
              className="group bg-bg-secondary/40 hover:bg-bg-secondary hover:border-border relative rounded-lg border border-transparent transition-all"
            >
              {editingId === rule.id ? (
                <div className="p-4">
                  <RuleForm
                    draft={editingDraft}
                    repos={repos}
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
                        {rule.text}
                      </div>
                      <button
                        onClick={() => onToggle(rule)}
                        className={`shrink-0 rounded-md px-2.5 py-1 text-[10px] font-semibold transition-colors ${
                          rule.enabled
                            ? 'bg-status-done/10 text-status-done border border-status-done/20'
                            : 'bg-bg-tertiary text-text-tertiary border border-border'
                        }`}
                      >
                        {rule.enabled ? 'Enabled' : 'Disabled'}
                      </button>
                    </div>

                    {/* Middle: Tags */}
                    <div className="text-text-tertiary flex flex-wrap items-center gap-2 text-[10px]">
                      <span className="bg-bg-tertiary rounded-md px-2 py-0.5 uppercase font-bold tracking-wider border border-border/50">
                        {rule.scope}
                      </span>
                      {rule.scope === 'repo' && (
                        <span className="bg-bg-tertiary rounded-md px-2 py-0.5 border border-border/50">
                          {repoName(rule.repo_id)}
                        </span>
                      )}
                      {rule.category && (
                        <span className="bg-bg-tertiary text-accent font-medium rounded-md px-2 py-0.5 border border-accent/20">
                          {rule.category.toUpperCase()}
                        </span>
                      )}
                      {rule.glob && (
                        <span className="bg-bg-tertiary rounded-md px-2 py-0.5 font-mono border border-border/50">
                          {rule.glob}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Bottom: Footer with Meta and Actions */}
                  <div className="border-t border-border/50 px-4 py-3 flex items-center justify-between">
                    <div className="text-text-tertiary flex items-center gap-4 text-[10px]">
                      <span className="flex items-center gap-1.5 opacity-80">
                        <Check size={12} className="text-status-done" />
                        Updated {new Date(rule.updated_at).toLocaleDateString()}
                      </span>
                      {statsById[rule.id] && (
                        <RuleEffectivenessIndicator stats={statsById[rule.id]} />
                      )}
                    </div>

                    <div className="flex items-center gap-1.5 opacity-0 transition-opacity group-hover:opacity-100">
                      <button
                        onClick={() => onEdit(rule)}
                        className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded-md p-1.5 transition-colors"
                        title="Edit rule"
                      >
                        <PencilSimple size={14} />
                      </button>
                      <button
                        onClick={() => onDelete(rule.id)}
                        className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 rounded-md p-1.5 transition-colors"
                        title="Delete rule"
                      >
                        <Trash size={14} />
                      </button>
                    </div>
                  </div>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
};

interface RuleEffectivenessIndicatorProps {
  stats: RuleRejectionStats;
}

const RuleEffectivenessIndicator: React.FC<RuleEffectivenessIndicatorProps> = ({ stats }) => {
  const acceptanceRate = 1 - stats.rejection_rate;
  const acceptancePercent = Math.round(acceptanceRate * 100);
  const isNoisy = stats.rejection_rate > 0.3; // >30% rejection rate is considered noisy

  return (
    <div className="flex items-center gap-3">
      <span className="flex items-center gap-1.5 opacity-80">
        <ChartBar size={12} className="text-text-disabled" />
        <span title={`${stats.total_feedback} feedback items generated`}>
          {stats.total_feedback} triggered
        </span>
      </span>
      <span className="text-text-disabled/40">·</span>
      <span
        className={`flex items-center gap-1.5 ${isNoisy ? 'text-status-ignored' : acceptancePercent >= 80 ? 'text-status-done' : 'text-status-in_progress'}`}
        title={`${acceptancePercent}% of feedback from this rule was accepted (${stats.rejected_count} rejected)`}
      >
        {isNoisy && <Warning size={12} />}
        {acceptancePercent}% accepted
      </span>
      {isNoisy && (
        <span className="bg-status-ignored/10 text-status-ignored border border-status-ignored/20 rounded-[2px] px-1.5 py-0.5 text-[9px] font-bold uppercase tracking-tighter">
          Noisy
        </span>
      )}
    </div>
  );
};
