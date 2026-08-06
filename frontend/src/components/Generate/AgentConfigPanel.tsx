import React from 'react';
import { motion } from 'framer-motion';
import * as Select from '@radix-ui/react-select';
import { CaretDown, Check } from '@phosphor-icons/react';
import { AgentSelector } from './AgentSelector';
import { Tooltip } from '../Common/Tooltip';
import { HarnessConfigControls } from './HarnessConfigControls';
import type { Agent, LinkedRepo } from '../../types';
import {
  configFieldLabelClass,
  configFieldTriggerClass,
  configSelectContentClass,
  configSelectItemClass,
} from './configFieldStyles';
import { WindowDragRegion } from '../Layout/WindowDragRegion';

const NO_REPOSITORY = '__none__';

interface AgentConfigPanelProps {
  agents: Agent[];
  repos: LinkedRepo[];
  selectedAgentId: string;
  selectedRepoId: string;
  onAgentSelect: (agentId: string) => void;
  onRepoSelect: (repoId: string) => void;
  isStarting: boolean;
  onGenerate: () => void;
  isDiffValid: boolean;
}

export const AgentConfigPanel: React.FC<AgentConfigPanelProps> = ({
  agents,
  repos,
  selectedAgentId,
  selectedRepoId,
  onAgentSelect,
  onRepoSelect,
  isStarting,
  onGenerate,
  isDiffValid,
}) => {
  return (
    <div
      data-testid="agent-config-strip"
      className="border-border bg-bg-secondary relative flex shrink-0 items-end gap-3 overflow-x-auto border-b px-4 py-3"
    >
      <WindowDragRegion className="absolute top-0 right-0 left-0 z-20 h-6" />

      <div className="w-[190px] shrink-0">
        <AgentSelector agents={agents} selectedAgentId={selectedAgentId} onSelect={onAgentSelect} />
      </div>

      <div className="shrink-0">
        <HarnessConfigControls
          agentId={selectedAgentId}
          enabled={agents.some(agent => agent.id === selectedAgentId && agent.available !== false)}
          disabled={isStarting}
          compact
        />
      </div>

      <div className="w-[190px] shrink-0 space-y-1.5">
        <label className={configFieldLabelClass}>Repository</label>
        <Select.Root
          value={selectedRepoId || NO_REPOSITORY}
          onValueChange={value => onRepoSelect(value === NO_REPOSITORY ? '' : value)}
          disabled={isStarting}
        >
          <Select.Trigger
            aria-label="Repository"
            className={`${configFieldTriggerClass} w-full justify-between`}
          >
            <Select.Value />
            <Select.Icon className="text-text-disabled">
              <CaretDown size={12} />
            </Select.Icon>
          </Select.Trigger>

          <Select.Portal>
            <Select.Content
              className={`${configSelectContentClass} min-w-[var(--radix-select-trigger-width)]`}
              position="popper"
              sideOffset={8}
              collisionPadding={12}
            >
              <Select.Viewport className="p-1">
                <Select.Item value={NO_REPOSITORY} className={configSelectItemClass}>
                  <Select.ItemText>None</Select.ItemText>
                  <Select.ItemIndicator className="text-brand absolute right-3">
                    <Check size={12} weight="bold" />
                  </Select.ItemIndicator>
                </Select.Item>
                {repos.map(repo => (
                  <Select.Item key={repo.id} value={repo.id} className={configSelectItemClass}>
                    <Select.ItemText>{repo.name}</Select.ItemText>
                    <Select.ItemIndicator className="text-brand absolute right-3">
                      <Check size={12} weight="bold" />
                    </Select.ItemIndicator>
                  </Select.Item>
                ))}
              </Select.Viewport>
            </Select.Content>
          </Select.Portal>
        </Select.Root>
      </div>

      <div className="min-w-3 flex-1" />

      {!isDiffValid && !isStarting ? (
        <Tooltip content="Please add a valid git diff to start a review">
          <motion.button
            disabled={true}
            className="bg-bg-tertiary border-border/50 text-text-disabled shadow-custom relative flex h-9 w-[160px] shrink-0 cursor-not-allowed items-center justify-center gap-2 overflow-hidden rounded-md border px-4 text-xs font-bold"
          >
            <div className="relative z-10 flex items-center justify-center gap-2">
              <PlayIcon size={14} />
              <span>Start Review</span>
            </div>
          </motion.button>
        </Tooltip>
      ) : (
        <motion.button
          whileTap={{ scale: 0.98 }}
          onClick={onGenerate}
          disabled={isStarting}
          className={`shadow-custom relative flex h-9 w-[160px] shrink-0 items-center justify-center gap-2 overflow-hidden rounded-md px-4 text-xs font-bold transition-[filter,background-color] ${
            isStarting
              ? 'bg-brand/70 text-bg-primary cursor-wait'
              : 'bg-brand text-bg-primary hover:brightness-110'
          }`}
        >
          {/* Iridescent shimmer overlay */}
          {isStarting && (
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
                scale: isStarting ? [1, 1.05, 1] : 1,
                rotate: isStarting ? [0, 90] : 0,
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
                    d: isStarting ? 'M6 6 L18 6 L18 18 L6 18 Z' : 'M8 5 L19 12 L8 19 Z', // Triangle (Play)
                  }}
                  transition={{
                    type: 'spring',
                    stiffness: 400,
                    damping: 25,
                  }}
                />
              </svg>
            </motion.div>
            <span>{isStarting ? 'Starting…' : 'Start Review'}</span>
          </div>

          {/* Sparkles */}
          {isStarting && <Sparkles />}
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
