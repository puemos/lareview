import React from 'react';
import { Database } from '@phosphor-icons/react';
import { AgentSelector } from './AgentSelector';
import type { Agent, LinkedRepo } from '../../types';

interface AgentConfigPanelProps {
  agents: Agent[];
  repos: LinkedRepo[];
  selectedAgentId: string;
  selectedRepoId: string;
  onAgentSelect: (agentId: string) => void;
  onRepoSelect: (repoId: string) => void;
  isGenerating: boolean;
  onGenerate: () => void;
  hasDiff: boolean;
}

export const AgentConfigPanel: React.FC<AgentConfigPanelProps> = ({
  agents,
  repos,
  selectedAgentId,
  selectedRepoId,
  onAgentSelect,
  onRepoSelect,
  isGenerating,
  onGenerate,
  hasDiff,
}) => {
  return (
    <div className="border-border space-y-6 border-b p-5">
      <div className="space-y-4">
        <AgentSelector agents={agents} selectedAgentId={selectedAgentId} onSelect={onAgentSelect} />

        <div className="space-y-1.5">
          <label className="text-text-disabled flex items-center gap-1.5 text-[10px] font-bold tracking-wider uppercase">
            <Database size={12} /> Repository
          </label>
          <div className="group relative">
            <select
              value={selectedRepoId}
              onChange={e => onRepoSelect(e.target.value)}
              className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 hover:border-text-disabled w-full cursor-pointer appearance-none rounded-md border py-2 pr-8 pl-3 text-xs transition-all focus:ring-1 focus:outline-none"
              disabled={isGenerating}
            >
              <option value="">None</option>
              {repos.map(r => (
                <option key={r.id} value={r.id}>
                  {r.name}
                </option>
              ))}
            </select>
            <div className="text-text-disabled group-hover:text-text-secondary pointer-events-none absolute top-1/2 right-2.5 -translate-y-1/2">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor">
                <path d="M7 10l5 5 5-5z" />
              </svg>
            </div>
          </div>
        </div>
      </div>

      <button
        onClick={onGenerate}
        disabled={!hasDiff || isGenerating}
        className={`shadow-custom flex w-full items-center justify-center gap-2 rounded-md py-2.5 text-xs font-bold transition-all active:scale-[0.98] ${
          isGenerating
            ? 'bg-status-ignored/10 text-status-ignored border-status-ignored/20 hover:bg-status-ignored/20 border'
            : 'bg-brand text-bg-primary hover:brightness-110'
        }`}
      >
        {isGenerating ? (
          <>
            <StopIcon size={14} />
            Stop Generation
          </>
        ) : (
          <>
            <PlayIcon size={14} />
            Generate Review
          </>
        )}
      </button>
    </div>
  );
};

const StopIcon = ({ size }: { size: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <rect x="6" y="6" width="12" height="12" rx="2" />
  </svg>
);

const PlayIcon = ({ size }: { size: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M8 5v14l11-7z" />
  </svg>
);
