import React from 'react';
import { motion } from 'framer-motion';
import { Database } from '@phosphor-icons/react';
import { AgentSelector } from './AgentSelector';
import { Tooltip } from '../Common/Tooltip';
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
  onStop: () => void;
  isDiffValid: boolean;
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
  onStop,
  isDiffValid,
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

      {!isDiffValid && !isGenerating ? (
        <Tooltip content="Please add a valid git diff to generate a review">
          <motion.button
            disabled={true}
            className="bg-bg-tertiary border-border/50 text-text-disabled shadow-custom relative flex w-full cursor-not-allowed items-center justify-center gap-2 overflow-hidden rounded-md border py-2.5 text-xs font-bold transition-all"
          >
            <div className="relative z-10 flex items-center justify-center gap-2">
              <PlayIcon size={14} />
              <span>Generate Review</span>
            </div>
          </motion.button>
        </Tooltip>
      ) : (
        <motion.button
          whileTap={{ scale: 0.98 }}
          onClick={isGenerating ? onStop : onGenerate}
          className={`shadow-custom relative flex w-full items-center justify-center gap-2 overflow-hidden rounded-md py-2.5 text-xs font-bold transition-all ${
            isGenerating
              ? 'bg-status-ignored/10 text-status-ignored border-status-ignored/20 hover:bg-status-ignored/20 cursor-pointer border'
              : 'bg-brand text-bg-primary hover:brightness-110'
          }`}
        >
          {/* Iridescent shimmer overlay */}
          {isGenerating && (
            <motion.div
              animate={{
                backgroundPosition: ['200% 50%', '-200% 50%'],
              }}
              transition={{
                duration: 4,
                repeat: Infinity,
                ease: 'linear',
              }}
              style={{
                background:
                  'linear-gradient(90deg, transparent, rgba(255,255,255,0.05), rgba(168,85,247,0.15), rgba(255,255,255,0.05), transparent)',
                backgroundSize: '200% 100%',
              }}
              className="absolute inset-0 z-0"
            />
          )}

          <div className="relative z-10 flex items-center justify-center gap-2">
            <motion.div
              animate={{
                scale: isGenerating ? [1, 1.1, 1] : 1,
                rotate: isGenerating ? [0, 90] : 0,
              }}
              transition={{
                type: 'spring',
                stiffness: 500,
                damping: 30,
              }}
              className="flex items-center justify-center"
            >
              <svg width="14" height="14" viewBox="0 0 24 24" className="fill-current">
                <motion.path
                  animate={{
                    d: isGenerating
                      ? 'M6 6 L18 6 L18 18 L6 18 Z' // Square (Stop)
                      : 'M8 5 L19 12 L8 19 Z', // Triangle (Play)
                  }}
                  transition={{
                    type: 'spring',
                    stiffness: 400,
                    damping: 25,
                  }}
                />
              </svg>
            </motion.div>
            <span>{isGenerating ? 'Stop Generation' : 'Generate Review'}</span>
          </div>

          {/* Sparkles */}
          {isGenerating && <Sparkles />}

          {/* Radial pulse */}
          {isGenerating && (
            <motion.div
              animate={{
                scale: [1, 1.2, 1],
                opacity: [0.3, 0.6, 0.3],
              }}
              transition={{
                duration: 2,
                repeat: Infinity,
                ease: 'easeInOut',
              }}
              className="absolute inset-0 rounded-full border border-current opacity-20"
            />
          )}
        </motion.button>
      )}
    </div>
  );
};

const Sparkles = () => {
  return (
    <div className="pointer-events-none absolute inset-0 z-10 overflow-hidden">
      {[...Array(6)].map((_, i) => (
        <motion.div
          key={i}
          animate={{
            x: [0, (Math.random() - 0.5) * 40],
            y: [0, (Math.random() - 0.5) * 40],
            opacity: [0, 1, 0],
            scale: [0, 1, 0],
          }}
          transition={{
            duration: 1 + Math.random() * 2,
            repeat: Infinity,
            delay: Math.random() * 2,
          }}
          className="absolute h-1 w-1 rounded-full bg-current"
          style={{
            left: `${10 + Math.random() * 80}%`,
            top: `${10 + Math.random() * 80}%`,
          }}
        />
      ))}
    </div>
  );
};

const PlayIcon = ({ size }: { size: number }) => (
  <svg width={size} height={size} viewBox="0 0 24 24" fill="currentColor">
    <path d="M8 5v14l11-7z" />
  </svg>
);
