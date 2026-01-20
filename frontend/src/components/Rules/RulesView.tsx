import React, { useMemo, useState } from 'react';
import { Asterisk, Check, PencilSimple, Trash } from '@phosphor-icons/react';
import { ICONS } from '../../constants/icons';
import { useRules, type ReviewRuleInput } from '../../hooks/useRules';
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

  const [draft, setDraft] = useState<RuleDraft>(emptyDraft);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isLibraryOpen, setIsLibraryOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingDraft, setEditingDraft] = useState<RuleDraft>(emptyDraft);

  const globalRules = useMemo(() => rules.filter(rule => rule.scope === 'global'), [rules]);
  const repoRules = useMemo(() => rules.filter(rule => rule.scope === 'repo'), [rules]);

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
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded p-1 transition-all"
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
        <div className="text-text-tertiary bg-bg-secondary/40 border-border rounded-md border p-4 text-xs">
          No rules configured yet.
        </div>
      ) : (
        <div className="space-y-3">
          {rules.map(rule => (
            <div key={rule.id} className="bg-bg-secondary/40 border-border rounded-lg border p-4">
              {editingId === rule.id ? (
                <RuleForm
                  draft={editingDraft}
                  repos={repos}
                  onChange={onEditChange}
                  onSubmit={onUpdate}
                  submitLabel="Save Changes"
                  onCancel={onCancelEdit}
                />
              ) : (
                <div className="space-y-3">
                  <div className="flex flex-wrap items-center justify-between gap-3">
                    <div className="space-y-1">
                      <div className="text-text-primary text-sm font-medium">{rule.text}</div>
                      <div className="text-text-tertiary flex flex-wrap items-center gap-2 text-[10px]">
                        <span className="bg-bg-tertiary rounded px-2 py-0.5 uppercase">
                          {rule.scope}
                        </span>
                        {rule.scope === 'repo' && (
                          <span className="bg-bg-tertiary rounded px-2 py-0.5">
                            {repoName(rule.repo_id)}
                          </span>
                        )}
                        {rule.category && (
                          <span className="bg-bg-tertiary rounded px-2 py-0.5">
                            {rule.category}
                          </span>
                        )}
                        {rule.glob && (
                          <span className="bg-bg-tertiary rounded px-2 py-0.5 font-mono">
                            {rule.glob}
                          </span>
                        )}
                      </div>
                    </div>

                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => onToggle(rule)}
                        className={`rounded-md px-2.5 py-1 text-[10px] font-semibold transition-colors ${
                          rule.enabled
                            ? 'bg-status-done/10 text-status-done'
                            : 'bg-bg-tertiary text-text-tertiary'
                        }`}
                      >
                        {rule.enabled ? 'Enabled' : 'Disabled'}
                      </button>
                      <button
                        onClick={() => onEdit(rule)}
                        className="text-text-tertiary hover:text-text-primary rounded-md border border-transparent p-1 transition-colors"
                        title="Edit rule"
                      >
                        <PencilSimple size={14} />
                      </button>
                      <button
                        onClick={() => onDelete(rule.id)}
                        className="text-text-tertiary hover:text-status-ignored rounded-md border border-transparent p-1 transition-colors"
                        title="Delete rule"
                      >
                        <Trash size={14} />
                      </button>
                    </div>
                  </div>

                  <div className="text-text-tertiary flex items-center gap-2 text-[10px]">
                    <Check size={12} className="text-status-done" />
                    Updated {new Date(rule.updated_at).toLocaleDateString()}
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
