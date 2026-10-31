import React, { useState } from 'react';
import { Folder, Plus, ArrowRight, Trash, SpinnerGap } from '@phosphor-icons/react';
import { PageHeader } from '../Common/PageHeader';
import { SearchField } from '../Common/SearchField';
import type { ViewType } from '../../types';
import { useRepos } from '../../hooks/useRepos';
import { useAppStore } from '../../store';
import { displayDate } from '../../utils/dates';
import { InlineError } from '../Common/InlineError';

interface ReposViewProps {
  onNavigate: (view: ViewType) => void;
}

export const ReposView: React.FC<ReposViewProps> = ({ onNavigate }) => {
  const [searchQuery, setSearchQuery] = useState('');
  const {
    data: repos = [],
    isLoading,
    isError,
    error,
    refetch,
    isFetching,
    addRepo,
    removeRepo,
    selectRepoFolder,
  } = useRepos();

  const normalizedQuery = searchQuery.trim().toLowerCase();
  const filteredRepos = repos.filter(
    repo =>
      repo.name.toLowerCase().includes(normalizedQuery) ||
      repo.path.toLowerCase().includes(normalizedQuery)
  );

  const handleAddRepo = async () => {
    const path = await selectRepoFolder();
    if (path) {
      addRepo.mutate(path);
    }
  };

  const startRepositoryReview = (repoId: string) => {
    useAppStore.setState({
      selectedRepoId: repoId,
      diffText: '',
      parsedDiff: null,
      pendingSource: null,
      prRef: '',
      viewMode: 'raw',
    });
    onNavigate('generate');
  };

  return (
    <div className="bg-bg-primary flex h-full flex-col">
      <PageHeader
        title="Repositories"
        description="Link local projects to give your reviews the right context."
        actions={
          <button
            onClick={handleAddRepo}
            disabled={addRepo.isPending}
            aria-busy={addRepo.isPending}
            className="bg-brand text-brand-fg hover:bg-brand/90 flex h-9 items-center gap-2 rounded-md px-3 text-xs font-medium transition-colors"
          >
            {addRepo.isPending ? (
              <SpinnerGap size={14} className="animate-spin" />
            ) : (
              <Plus size={14} />
            )}{' '}
            {addRepo.isPending ? 'Linking…' : 'Link repository'}
          </button>
        }
      />

      <div className="product-page-body">
        <div className="product-page-content space-y-6">
          <div className="flex flex-wrap items-center justify-between gap-3">
            <SearchField
              label="Search repositories"
              value={searchQuery}
              onChange={setSearchQuery}
            />
            {!isLoading && !isError && (
              <span role="status" className="text-text-disabled text-xs">
                {filteredRepos.length} {filteredRepos.length === 1 ? 'repository' : 'repositories'}
              </span>
            )}
          </div>

          {isError ? (
            <InlineError
              title="Could not load repositories"
              message={String(error)}
              onRetry={() => {
                void refetch();
              }}
              isRetrying={isFetching}
            />
          ) : isLoading ? (
            <div
              role="status"
              aria-label="Loading repositories"
              className="repository-grid animate-pulse"
            >
              {[1, 2, 3].map(index => (
                <div
                  key={index}
                  aria-hidden="true"
                  className="border-border/50 space-y-4 rounded-lg border p-5"
                >
                  <div className="bg-bg-tertiary h-9 w-9 rounded-md" />
                  <div className="bg-bg-tertiary h-4 w-1/2 rounded" />
                  <div className="bg-bg-tertiary h-3 w-3/4 rounded" />
                  <div className="bg-bg-tertiary h-3 w-1/3 rounded" />
                </div>
              ))}
            </div>
          ) : filteredRepos.length === 0 ? (
            <div className="text-text-disabled border-border bg-bg-secondary/30 flex flex-col items-center justify-center rounded-lg border border-dashed px-6 py-14 text-center">
              <Folder size={32} className="mb-3 opacity-50" />
              <p className="text-text-primary mb-2 text-sm font-medium">
                {normalizedQuery ? 'No matching repositories' : 'Link your first repository'}
              </p>
              <p className="text-text-tertiary mb-6 text-xs">
                {normalizedQuery
                  ? 'Search by repository name or local path.'
                  : 'Get started by linking your first repository'}
              </p>
              <button
                onClick={normalizedQuery ? () => setSearchQuery('') : handleAddRepo}
                disabled={addRepo.isPending}
                className="bg-bg-tertiary text-text-primary hover:bg-bg-primary border-border rounded border px-4 py-2 text-xs font-medium shadow-sm transition-colors"
              >
                {normalizedQuery ? 'Clear search' : 'Link repository'}
              </button>
            </div>
          ) : (
            <div className="repository-grid">
              {filteredRepos.map(repo => (
                <RepoCard
                  key={repo.id}
                  repo={repo}
                  onReview={() => startRepositoryReview(repo.id)}
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
  onReview: () => void;
  onRemove: (repoId: string) => void;
}

const RepoCard: React.FC<RepoCardProps> = ({ repo, onReview, onRemove }) => (
  <div className="group bg-bg-secondary/25 border-border/50 hover:border-border relative min-w-0 rounded-lg border p-5 transition-colors">
    <div className="mb-3 flex items-start justify-between">
      <div className="bg-bg-tertiary/60 text-text-secondary rounded-md p-2">
        <Folder size={20} />
      </div>
      <div className="flex items-center gap-2">
        <button
          onClick={onReview}
          className="text-text-secondary hover:text-text-primary hover:bg-bg-tertiary border-border/60 flex h-8 items-center gap-2 rounded-md border px-2.5 text-xs transition-colors"
          aria-label={`Review changes in ${repo.name}`}
        >
          Review changes <ArrowRight size={13} />
        </button>
        <button
          onClick={() => onRemove(repo.id)}
          className="text-text-disabled hover:text-status-ignored hover:bg-bg-tertiary flex h-8 w-8 items-center justify-center rounded-md transition-colors"
          title="Unlink repository"
          aria-label={`Unlink ${repo.name}`}
        >
          <Trash size={14} />
        </button>
      </div>
    </div>

    <div>
      <h3 className="text-text-primary mb-1 truncate text-[0.9375rem] font-medium">{repo.name}</h3>
      <p
        title={repo.path}
        className="text-text-tertiary mb-4 line-clamp-2 font-mono text-[0.6875rem] leading-5 break-all select-text"
      >
        {repo.path}
      </p>

      <div className="border-border/50 mt-2 flex w-full items-center gap-3 border-t pt-3">
        {displayDate(repo.linked_at) && (
          <time dateTime={repo.linked_at} className="text-text-disabled text-[0.6875rem]">
            Linked {displayDate(repo.linked_at)}
          </time>
        )}
      </div>
    </div>
  </div>
);
