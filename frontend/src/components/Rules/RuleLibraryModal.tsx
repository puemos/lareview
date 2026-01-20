import React, { useState } from 'react';
import { ICONS } from '../../constants/icons';
import { useRuleLibrary } from '../../hooks/useRuleLibrary';
import type { LibraryRule, LibraryCategory, RuleScope, LinkedRepo } from '../../types';

interface RuleLibraryModalProps {
  isOpen: boolean;
  onClose: () => void;
  repos: LinkedRepo[];
}

const CATEGORY_INFO: Record<LibraryCategory, { label: string; description: string }> = {
  security: { label: 'Security', description: 'Authentication, injection, secrets' },
  code_quality: { label: 'Code Quality', description: 'Clean code principles' },
  testing: { label: 'Testing', description: 'Test coverage and practices' },
  documentation: { label: 'Documentation', description: 'Comments and docs' },
  performance: { label: 'Performance', description: 'Optimization concerns' },
  api_design: { label: 'API Design', description: 'Interface patterns' },
  language_specific: { label: 'Language', description: 'Language-specific rules' },
  framework_specific: { label: 'Framework', description: 'Framework-specific rules' },
};

const CATEGORY_ORDER: LibraryCategory[] = [
  'security',
  'code_quality',
  'testing',
  'performance',
  'api_design',
  'documentation',
  'language_specific',
  'framework_specific',
];

export const RuleLibraryModal: React.FC<RuleLibraryModalProps> = ({ isOpen, onClose, repos }) => {
  const { allRules, addFromLibrary } = useRuleLibrary();
  const [selectedCategory, setSelectedCategory] = useState<LibraryCategory | 'all'>('all');
  const [addingRuleId, setAddingRuleId] = useState<string | null>(null);
  const [addScope, setAddScope] = useState<RuleScope>('global');
  const [addRepoId, setAddRepoId] = useState<string>('');

  if (!isOpen) return null;

  const rules = allRules.data || [];

  const filteredRules =
    selectedCategory === 'all'
      ? rules
      : rules.filter(r => r.library_category === selectedCategory);

  const rulesByCategory = CATEGORY_ORDER.reduce(
    (acc, cat) => {
      const catRules = rules.filter(r => r.library_category === cat);
      if (catRules.length > 0) {
        acc[cat] = catRules;
      }
      return acc;
    },
    {} as Record<LibraryCategory, LibraryRule[]>
  );

  const handleAddRule = (rule: LibraryRule) => {
    setAddingRuleId(rule.id);
  };

  const confirmAddRule = () => {
    if (!addingRuleId) return;
    addFromLibrary.mutate(
      {
        libraryRuleId: addingRuleId,
        scope: addScope,
        repoId: addScope === 'repo' ? addRepoId : undefined,
      },
      {
        onSuccess: () => {
          setAddingRuleId(null);
          setAddScope('global');
          setAddRepoId('');
        },
      }
    );
  };

  const cancelAddRule = () => {
    setAddingRuleId(null);
    setAddScope('global');
    setAddRepoId('');
  };

  const addingRule = rules.find(r => r.id === addingRuleId);

  return (
    <div className="animate-in fade-in fixed inset-0 z-50 flex items-center justify-center bg-black/60 p-4 backdrop-blur-sm duration-200">
      <div className="bg-bg-primary border-border/50 animate-in zoom-in-95 flex h-[80vh] w-full max-w-4xl flex-col rounded-xl border shadow-2xl duration-200">
        {/* Header */}
        <div className="border-border/50 bg-bg-secondary/30 flex items-center justify-between rounded-t-xl border-b px-5 py-4">
          <div className="flex items-center gap-2.5">
            <div className="bg-brand/10 text-brand rounded-md p-1.5">
              <ICONS.ICON_PLAN size={18} />
            </div>
            <div>
              <h3 className="text-text-primary text-sm font-semibold">Rule Library</h3>
              <p className="text-text-tertiary text-xs">
                Pre-built rules to get started quickly
              </p>
            </div>
          </div>
          <button
            onClick={onClose}
            className="text-text-tertiary hover:text-text-primary hover:bg-bg-tertiary rounded p-1 transition-all"
          >
            <ICONS.ACTION_CLOSE size={18} />
          </button>
        </div>

        {/* Content */}
        <div className="flex flex-1 overflow-hidden">
          {/* Category Sidebar */}
          <div className="border-border/50 w-48 flex-shrink-0 overflow-y-auto border-r p-3">
            <button
              onClick={() => setSelectedCategory('all')}
              className={`mb-1 w-full rounded-md px-3 py-2 text-left text-xs transition-colors ${
                selectedCategory === 'all'
                  ? 'bg-brand/10 text-brand'
                  : 'text-text-secondary hover:bg-bg-tertiary hover:text-text-primary'
              }`}
            >
              All Rules ({rules.length})
            </button>
            <div className="border-border/30 my-2 border-t" />
            {CATEGORY_ORDER.map(cat => {
              const catRules = rulesByCategory[cat];
              if (!catRules) return null;
              const info = CATEGORY_INFO[cat];
              return (
                <button
                  key={cat}
                  onClick={() => setSelectedCategory(cat)}
                  className={`mb-1 w-full rounded-md px-3 py-2 text-left transition-colors ${
                    selectedCategory === cat
                      ? 'bg-brand/10 text-brand'
                      : 'text-text-secondary hover:bg-bg-tertiary hover:text-text-primary'
                  }`}
                >
                  <div className="text-xs font-medium">{info.label}</div>
                  <div className="text-[10px] opacity-70">
                    {catRules.length} {catRules.length === 1 ? 'rule' : 'rules'}
                  </div>
                </button>
              );
            })}
          </div>

          {/* Rules List */}
          <div className="flex-1 overflow-y-auto p-4">
            {allRules.isLoading ? (
              <div className="animate-pulse space-y-3">
                {[1, 2, 3, 4].map(i => (
                  <div key={i} className="bg-bg-tertiary/50 h-24 rounded-lg" />
                ))}
              </div>
            ) : filteredRules.length === 0 ? (
              <div className="text-text-tertiary py-12 text-center text-sm">
                No rules in this category.
              </div>
            ) : (
              <div className="space-y-3">
                {filteredRules.map(rule => (
                  <RuleCard
                    key={rule.id}
                    rule={rule}
                    onAdd={() => handleAddRule(rule)}
                    isAdding={addFromLibrary.isPending && addingRuleId === rule.id}
                  />
                ))}
              </div>
            )}
          </div>
        </div>

        {/* Add Rule Confirmation */}
        {addingRule && (
          <div className="border-border/50 bg-bg-secondary/50 border-t p-4">
            <div className="mb-3 flex items-center gap-2">
              <ICONS.ICON_PLUS size={14} className="text-brand" />
              <span className="text-text-primary text-sm font-medium">
                Add &quot;{addingRule.name}&quot; to your rules
              </span>
            </div>
            <div className="flex items-center gap-3">
              <select
                value={addScope}
                onChange={e => setAddScope(e.target.value as RuleScope)}
                className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
              >
                <option value="global">Global</option>
                <option value="repo">Repository</option>
              </select>
              {addScope === 'repo' && (
                <select
                  value={addRepoId}
                  onChange={e => setAddRepoId(e.target.value)}
                  className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 flex-1 rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
                >
                  <option value="">Select a repository</option>
                  {repos.map(repo => (
                    <option key={repo.id} value={repo.id}>
                      {repo.name}
                    </option>
                  ))}
                </select>
              )}
              <div className="ml-auto flex items-center gap-2">
                <button
                  onClick={cancelAddRule}
                  className="bg-bg-tertiary text-text-secondary hover:text-text-primary border-border rounded-md border px-3 py-2 text-xs font-medium transition-all"
                >
                  Cancel
                </button>
                <button
                  onClick={confirmAddRule}
                  disabled={addScope === 'repo' && !addRepoId}
                  className="bg-brand text-bg-primary disabled:bg-bg-tertiary disabled:text-text-disabled rounded-md px-3 py-2 text-xs font-semibold transition-all hover:brightness-110 disabled:cursor-not-allowed"
                >
                  {addFromLibrary.isPending ? 'Adding...' : 'Add Rule'}
                </button>
              </div>
            </div>
          </div>
        )}
      </div>
    </div>
  );
};

interface RuleCardProps {
  rule: LibraryRule;
  onAdd: () => void;
  isAdding: boolean;
}

const RuleCard: React.FC<RuleCardProps> = ({ rule, onAdd, isAdding }) => {
  const [expanded, setExpanded] = useState(false);

  return (
    <div className="bg-bg-secondary/40 border-border/50 rounded-lg border p-4">
      <div className="flex items-start justify-between gap-3">
        <div className="min-w-0 flex-1">
          <div className="flex items-center gap-2">
            <h4 className="text-text-primary text-sm font-medium">{rule.name}</h4>
            {rule.category && (
              <span className="bg-bg-tertiary text-text-tertiary rounded px-1.5 py-0.5 text-[9px] font-bold">
                {rule.category}
              </span>
            )}
          </div>
          <p className="text-text-secondary mt-1 text-xs">{rule.description}</p>
          <div className="mt-2 flex flex-wrap items-center gap-1.5">
            {rule.tags.slice(0, 3).map(tag => (
              <span
                key={tag}
                className="bg-bg-tertiary text-text-tertiary rounded px-1.5 py-0.5 text-[10px]"
              >
                {tag}
              </span>
            ))}
            {rule.glob && (
              <span className="bg-bg-tertiary text-text-tertiary rounded px-1.5 py-0.5 font-mono text-[10px]">
                {rule.glob}
              </span>
            )}
          </div>
        </div>
        <button
          onClick={onAdd}
          disabled={isAdding}
          className="bg-brand/10 text-brand hover:bg-brand/20 flex-shrink-0 rounded-md px-3 py-1.5 text-xs font-medium transition-colors disabled:opacity-50"
        >
          {isAdding ? 'Adding...' : 'Add'}
        </button>
      </div>

      {/* Expandable rule text */}
      <div className="mt-3">
        <button
          onClick={() => setExpanded(!expanded)}
          className="text-text-tertiary hover:text-text-secondary flex items-center gap-1 text-[10px] transition-colors"
        >
          <ICONS.CHEVRON_DOWN
            size={10}
            className={`transition-transform ${expanded ? 'rotate-180' : ''}`}
          />
          {expanded ? 'Hide rule text' : 'Show rule text'}
        </button>
        {expanded && (
          <div className="bg-bg-tertiary/50 text-text-secondary mt-2 rounded-md p-3 font-mono text-xs whitespace-pre-wrap">
            {rule.text}
          </div>
        )}
      </div>
    </div>
  );
};
