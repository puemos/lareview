import React, { useState, useEffect } from 'react';
import {
  Folder,
  Plus,
  MagnifyingGlass,
  ArrowSquareOut,
  Trash,
  Asterisk,
} from '@phosphor-icons/react';
import type { ViewType } from '../../types';
import { useRepos } from '../../hooks/useRepos';

interface ReposViewProps {
  onNavigate: (view: ViewType) => void;
}

export const ReposView: React.FC<ReposViewProps> = ({ onNavigate }) => {
  const [searchQuery, setSearchQuery] = useState('');
  const { data: repos = [], isLoading, addRepo, removeRepo, selectRepoFolder } = useRepos();

  useEffect(() => {
    if (addRepo.isError) {
      alert(`Failed to link repository: ${addRepo.error}`);
    }
  }, [addRepo.isError, addRepo.error]);

  const filteredRepos = repos.filter(
    repo =>
      repo.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
      repo.path.toLowerCase().includes(searchQuery.toLowerCase())
  );

  const handleAddRepo = async () => {
    const path = await selectRepoFolder();
    if (path) {
      addRepo.mutate(path);
    }
  };

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      {/* Header */}
      <div className="border-border bg-bg-primary flex h-12 shrink-0 items-center justify-between border-b px-6">
        <div className="flex items-center gap-3">
          <Asterisk size={18} weight="fill" className="text-brand" />
          <h1 className="font-display text-text-primary text-sm font-medium tracking-wide">
            Repositories
          </h1>
        </div>
        <button
          onClick={handleAddRepo}
          className="bg-brand text-bg-primary shadow-custom flex items-center gap-1.5 rounded-md px-3 py-1.5 text-[10px] font-bold transition-all hover:brightness-110"
        >
          <Plus size={12} weight="bold" />
          Link Repository
        </button>
      </div>

      <div className="flex-1 overflow-y-auto p-8">
        <div className="mx-auto max-w-5xl space-y-6">
          <div className="group relative max-w-md">
            <MagnifyingGlass
              size={14}
              className="text-text-disabled group-focus-within:text-brand absolute top-1/2 left-3 -translate-y-1/2 transition-colors"
            />
            <input
              type="text"
              value={searchQuery}
              onChange={e => setSearchQuery(e.target.value)}
              placeholder="Search repositories..."
              className="bg-bg-tertiary border-border text-text-primary placeholder-text-disabled focus:border-brand focus:ring-brand/20 w-full rounded-md border py-2 pr-4 pl-9 text-xs shadow-sm transition-all focus:ring-1 focus:outline-none"
            />
          </div>

          {isLoading ? (
            <div className="flex items-center justify-center py-20 opacity-50">
              <div className="border-brand h-5 w-5 animate-spin rounded-full border-2 border-t-transparent" />
            </div>
          ) : filteredRepos.length === 0 ? (
            <div className="text-text-disabled border-border bg-bg-secondary/30 flex flex-col items-center justify-center rounded-lg border border-dashed py-24">
              <Folder size={32} className="mb-3 opacity-50" />
              <p className="mb-3 text-sm font-medium">No repositories found</p>
              <p className="text-text-tertiary mb-6 text-xs">
                {searchQuery
                  ? 'Try adjusting your search terms'
                  : 'Get started by linking your first repository'}
              </p>
              <button
                onClick={handleAddRepo}
                className="bg-bg-tertiary text-text-primary hover:bg-bg-primary border-border rounded border px-4 py-2 text-xs font-medium shadow-sm transition-colors"
              >
                Link Repository
              </button>
            </div>
          ) : (
            <div className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3">
              {filteredRepos.map(repo => (
                <RepoCard
                  key={repo.id}
                  repo={repo}
                  onNavigate={onNavigate}
                  onRemove={removeRepo.mutate}
                />
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};

interface RepoCardProps {
  repo: {
    id: string;
    name: string;
    path: string;
    review_count: number;
    linked_at: string;
  };
  onNavigate: (view: ViewType) => void;
  onRemove: (repoId: string) => void;
}

const RepoCard: React.FC<RepoCardProps> = ({ repo, onNavigate, onRemove }) => (
  <div className="group bg-bg-secondary/40 hover:bg-bg-secondary hover:border-border relative rounded-lg border border-transparent p-4 transition-all">
    <div className="mb-3 flex items-start justify-between">
      <div className="bg-bg-tertiary text-brand rounded-md p-2">
        <Folder size={20} weight="fill" />
      </div>
      <div className="flex items-center gap-1 opacity-0 transition-opacity group-hover:opacity-100">
        <button
          onClick={() => onNavigate('review')}
          className="text-text-tertiary hover:text-brand hover:bg-bg-tertiary rounded-md p-1.5 transition-colors"
          title="Review"
        >
          <ArrowSquareOut size={14} />
        </button>
        <button
          onClick={() => onRemove(repo.id)}
          className="text-text-tertiary hover:text-status-ignored hover:bg-bg-tertiary rounded-md p-1.5 transition-colors"
          title="Remove"
        >
          <Trash size={14} />
        </button>
      </div>
    </div>

    <div>
      <h3 className="text-text-primary group-hover:text-brand mb-1 truncate text-sm font-medium transition-colors">
        {repo.name}
      </h3>
      <p className="text-text-tertiary bg-bg-primary/50 border-border/50 mb-3 w-fit max-w-full truncate rounded border px-1.5 py-0.5 font-mono text-[10px]">
        {repo.path}
      </p>

      <div className="border-border/50 mt-2 flex w-full items-center gap-3 border-t pt-3">
        <div className="flex items-center gap-1.5">
          <span
            className={`h-1.5 w-1.5 rounded-full ${repo.review_count > 0 ? 'bg-status-done' : 'bg-status-todo'}`}
          />
          <span className="text-text-secondary text-[10px]">{repo.review_count} reviews</span>
        </div>
        <span className="text-text-disabled ml-auto text-[10px]">
          Added {new Date(repo.linked_at).toLocaleDateString()}
        </span>
      </div>
    </div>
  </div>
);
