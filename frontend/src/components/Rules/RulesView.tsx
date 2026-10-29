import React, { useMemo, useState } from 'react';
import { PencilSimple, Trash, Warning, ChartBar } from '@phosphor-icons/react';
import { ICONS } from '../../constants/icons';
import { PageHeader } from '../Common/PageHeader';
import { displayDate } from '../../utils/dates';
import { SearchField } from '../Common/SearchField';
import { EmptyPanel } from '../Common/EmptyPanel';
import { Toggle } from '../Common/Toggle';
import { Dialog } from '../Common/Dialog';
import { InlineError } from '../Common/InlineError';
import {
  useRules,
  useRuleRejectionStats,
  type ReviewRuleInput,
  type RuleRejectionStats,
} from '../../hooks/useRules';
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
  const {
    data: rules = [],
    isLoading,
    isError,
    error,
    refetch,
    isFetching,
    createRule,
    updateRule,
    removeRule,
  } = useRules();
  const { data: repos = [] } = useRepos();
  const { data: rejectionStats = [] } = useRuleRejectionStats();

  const [ruleToDelete, setRuleToDelete] = useState<ReviewRule | null>(null);
  const [searchQuery, setSearchQuery] = useState('');
  const [draft, setDraft] = useState<RuleDraft>(emptyDraft);
  const [isAddModalOpen, setIsAddModalOpen] = useState(false);
  const [isLibraryOpen, setIsLibraryOpen] = useState(false);
  const [editingId, setEditingId] = useState<string | null>(null);
  const [editingDraft, setEditingDraft] = useState<RuleDraft>(emptyDraft);

  const filteredRules = useMemo(() => {
    const query = searchQuery.trim().toLowerCase();
    return rules.filter(rule =>
      [
        rule.text,
        rule.category ?? '',
        rule.glob ?? '',
        repos.find(repo => repo.id === rule.repo_id)?.name ?? '',
      ].some(value => value.toLowerCase().includes(query))
    );
  }, [rules, repos, searchQuery]);
  const globalRules = filteredRules.filter(rule => rule.scope === 'global');
  const repoRules = filteredRules.filter(rule => rule.scope === 'repo');

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
      <PageHeader
        title="Rules"
        description="Define what your agents should check during every review."
        actions={
          <>
            <button
              type="button"
              onClick={() => setIsLibraryOpen(true)}
              className="bg-bg-secondary border-border text-text-secondary hover:text-text-primary flex h-9 items-center gap-2 rounded-md border px-3 text-xs transition-colors"
            >
              <ICONS.ICON_PLAN size={14} /> Library
            </button>
            <button
              type="button"
              onClick={openAddModal}
              className="bg-brand text-brand-fg hover:bg-brand/90 flex h-9 items-center gap-2 rounded-md px-3 text-xs font-medium transition-colors"
            >
              <ICONS.ICON_PLUS size={14} /> Add rule
            </button>
          </>
        }
      />

      <div className="product-page-body">
        <div className="product-page-content space-y-6">
          {isError && (
            <InlineError
              title="Could not load rules"
              message={String(error)}
              onRetry={() => {
                void refetch();
              }}
              isRetrying={isFetching}
            />
          )}
          <div className="flex flex-wrap items-center justify-between gap-3">
            <SearchField
              label="Search rules"
              value={searchQuery}
              onChange={setSearchQuery}
              placeholder="Rule text, category, project, or file pattern"
            />
            {!isLoading && !isError && (
              <span className="text-text-disabled text-xs">
                {filteredRules.length} {filteredRules.length === 1 ? 'rule' : 'rules'}
              </span>
            )}
          </div>
          {!isLoading && !isError && searchQuery.trim() && filteredRules.length === 0 && (
            <EmptyPanel
              title="No matching rules"
              description="Try another phrase, category, project, or file pattern."
              action={
                <button
                  type="button"
                  onClick={() => setSearchQuery('')}
                  className="text-text-secondary border-border rounded-md border px-3 py-2 text-xs"
                >
                  Clear search
                </button>
              }
            />
          )}

          {!isError && (!searchQuery.trim() || globalRules.length > 0) && (
            <RuleSection
              title="Global rules"
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
              onDelete={id => {
                removeRule.reset();
                setRuleToDelete(rules.find(rule => rule.id === id) ?? null);
              }}
              repoName={repoName}
              isLoading={isLoading}
              isUpdating={updateRule.isPending || removeRule.isPending}
            />
          )}
          {!isError && (!searchQuery.trim() || repoRules.length > 0) && (
            <RuleSection
              title="Repository rules"
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
              onDelete={id => {
                removeRule.reset();
                setRuleToDelete(rules.find(rule => rule.id === id) ?? null);
              }}
              repoName={repoName}
              isLoading={isLoading}
              isUpdating={updateRule.isPending || removeRule.isPending}
            />
          )}
        </div>
      </div>

      <Dialog
        isOpen={Boolean(ruleToDelete)}
        onClose={() => setRuleToDelete(null)}
        title="Delete rule"
        busy={removeRule.isPending}
      >
        <p className="text-text-secondary text-sm leading-6">
          Remove this check from future reviews?
        </p>
        <blockquote className="text-text-primary border-border mt-4 border-l-2 pl-4 text-sm leading-6">
          {ruleToDelete?.text}
        </blockquote>
        {removeRule.error && (
          <div className="mt-4">
            <InlineError title="Could not delete rule" message={String(removeRule.error)} />
          </div>
        )}
        <div className="mt-6 flex justify-end gap-3">
          <button
            type="button"
            onClick={() => setRuleToDelete(null)}
            disabled={removeRule.isPending}
            className="text-text-secondary rounded-md px-3 py-2 text-xs"
          >
            Cancel
          </button>
          <button
            type="button"
            disabled={removeRule.isPending}
            onClick={() => {
              if (ruleToDelete)
                removeRule.mutate(ruleToDelete.id, { onSuccess: () => setRuleToDelete(null) });
            }}
            className="bg-impact-blocking text-brand-fg rounded-md px-3 py-2 text-xs font-medium"
          >
            {removeRule.isPending ? 'Deleting…' : 'Delete rule'}
          </button>
        </div>
      </Dialog>

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
  return (
    <Dialog
      isOpen={isOpen}
      onClose={onClose}
      title="Add rule"
      description="Give your agent a specific, actionable check."
      busy={isSubmitting}
    >
      <RuleForm
        draft={draft}
        repos={repos}
        onChange={onChange}
        onSubmit={onSubmit}
        submitLabel="Add rule"
        disabled={!canSubmit}
        isLoading={isSubmitting}
        onCancel={onClose}
      />
    </Dialog>
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
  autoFocus?: boolean;
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
  autoFocus = false,
}) => {
  const helpId = React.useId();
  const repoRequired = draft.scope === 'repo' && repos.length === 0;

  return (
    <form
      className="product-form"
      onSubmit={event => {
        event.preventDefault();
        if (!disabled && !repoRequired && !isLoading) onSubmit();
      }}
    >
      <fieldset disabled={isLoading} className="min-w-0 space-y-4">
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
              <option value="global">Every repository</option>
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
              Category (optional)
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
              File pattern (optional)
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
            Rule text
          </span>
          <textarea
            value={draft.text}
            required
            autoFocus={autoFocus}
            data-autofocus
            aria-describedby={helpId}
            onChange={e => onChange({ ...draft, text: e.target.value })}
            rows={4}
            placeholder="Describe what the AI should check for and report on."
            className="bg-bg-tertiary border-border text-text-primary placeholder-text-disabled focus:border-brand focus:ring-brand/20 w-full resize-none rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
          />
        </label>

        <p id={helpId} className="text-text-tertiary text-xs leading-5">
          Write a specific check the agent can verify. Leave the file pattern blank to check every
          file.
        </p>

        <label className="text-text-secondary flex items-center gap-2 text-xs">
          <input
            type="checkbox"
            checked={draft.enabled}
            onChange={e => onChange({ ...draft, enabled: e.target.checked })}
            className="accent-brand"
          />
          Use this rule in new reviews
        </label>

        <div className="flex items-center gap-2">
          <button
            type="submit"
            disabled={disabled || repoRequired || isLoading}
            className="bg-brand text-bg-primary disabled:bg-bg-tertiary disabled:text-text-disabled rounded-md px-3 py-2 text-xs font-semibold transition-all hover:brightness-110 disabled:cursor-not-allowed"
          >
            {isLoading ? 'Saving…' : submitLabel}
          </button>
          {onCancel && (
            <button
              type="button"
              onClick={onCancel}
              className="bg-bg-tertiary text-text-secondary hover:text-text-primary border-border rounded-md border px-3 py-2 text-xs font-semibold transition-all"
            >
              Cancel
            </button>
          )}
        </div>
      </fieldset>
    </form>
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
  isUpdating: boolean;
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
  isUpdating,
}) => {
  return (
    <div>
      <div className="mb-3 flex items-center justify-between">
        <h3 className="text-text-primary text-[13px] font-medium">{title}</h3>
        <span className="text-text-disabled text-xs">
          {isLoading
            ? 'Loading…'
            : `${rules.filter(rule => rule.enabled).length} enabled · ${rules.length} ${rules.length === 1 ? 'rule' : 'rules'}`}
        </span>
      </div>

      {isLoading ? (
        <div
          role="status"
          aria-label={`Loading ${title.toLowerCase()}`}
          className="animate-pulse space-y-3"
        >
          {[1, 2].map(index => (
            <div
              key={index}
              aria-hidden="true"
              className="bg-bg-secondary border-border/50 h-28 rounded-lg border"
            />
          ))}
        </div>
      ) : rules.length === 0 ? (
        <div className="text-text-tertiary bg-bg-secondary/40 border-border rounded-lg border p-4 text-xs">
          {title === 'Global rules'
            ? 'No global rules yet. These apply to every review.'
            : 'No repository rules yet. Add one to tailor checks to a project.'}
        </div>
      ) : (
        <div className="space-y-3">
          {rules.map(rule => (
            <div
              key={rule.id}
              className="group bg-bg-secondary/20 border-border/50 relative overflow-hidden rounded-lg border transition-colors"
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
                    autoFocus
                    disabled={
                      !editingDraft.text.trim() ||
                      (editingDraft.scope === 'repo' && !editingDraft.repo_id)
                    }
                    isLoading={isUpdating}
                  />
                </div>
              ) : (
                <div className="flex flex-col">
                  {/* Top: Description and Toggle */}
                  <div className="space-y-3 p-4">
                    <div className="flex items-start justify-between gap-4">
                      <div className="text-text-primary flex-1 text-sm leading-6">{rule.text}</div>
                      <Toggle
                        label={`Rule: ${rule.text}`}
                        checked={rule.enabled}
                        onChange={() => onToggle(rule)}
                        disabled={isUpdating}
                      />
                    </div>

                    {/* Middle: Tags */}
                    <div className="text-text-tertiary flex flex-wrap items-center gap-2 text-[11px]">
                      {rule.scope === 'repo' && (
                        <span className="bg-bg-tertiary border-border/50 rounded-md border px-2 py-0.5">
                          {repoName(rule.repo_id)}
                        </span>
                      )}
                      {rule.category && (
                        <span className="text-text-tertiary border-border/50 rounded-md border px-2 py-0.5 text-[11px]">
                          {rule.category}
                        </span>
                      )}
                      {rule.glob && (
                        <span className="bg-bg-tertiary border-border/50 rounded-md border px-2 py-0.5 font-mono">
                          {rule.glob}
                        </span>
                      )}
                    </div>
                  </div>

                  {/* Bottom: Footer with Meta and Actions */}
                  <div className="border-border/50 flex items-center justify-between border-t px-4 py-3">
                    <div className="text-text-tertiary flex items-center gap-4 text-[10px]">
                      {displayDate(rule.updated_at) && (
                        <time dateTime={rule.updated_at}>
                          Updated {displayDate(rule.updated_at)}
                        </time>
                      )}
                      {statsById[rule.id] && (
                        <RuleEffectivenessIndicator stats={statsById[rule.id]} />
                      )}
                    </div>

                    <div className="flex items-center gap-2">
                      <button
                        onClick={() => onEdit(rule)}
                        disabled={isUpdating}
                        className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary flex h-8 items-center gap-1.5 rounded-md px-2 text-xs transition-colors"
                        aria-label={`Edit rule: ${rule.text}`}
                      >
                        <PencilSimple size={13} /> Edit
                      </button>
                      <button
                        onClick={() => onDelete(rule.id)}
                        disabled={isUpdating}
                        className="text-text-tertiary hover:text-status-ignored hover:bg-status-ignored/10 rounded-md p-1.5 transition-colors"
                        title="Delete rule"
                        aria-label={`Delete rule: ${rule.text}`}
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
        <span className="bg-status-ignored/10 text-status-ignored border-status-ignored/20 rounded-[2px] border px-1.5 py-0.5 text-[9px] font-bold tracking-tighter uppercase">
          Noisy
        </span>
      )}
    </div>
  );
};
