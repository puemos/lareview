import React from 'react';
import { Play, SpinnerGap } from '@phosphor-icons/react';
import * as Select from '@radix-ui/react-select';
import { CaretDown, Check } from '@phosphor-icons/react';
import { AgentSelector } from './AgentSelector';
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
  isLoadingSource?: boolean;
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
  isLoadingSource = false,
  onGenerate,
  isDiffValid,
}) => {
  const canGenerate =
    isDiffValid && agents.some(agent => agent.id === selectedAgentId && agent.available !== false);
  return (
    <div
      data-testid="agent-config-strip"
      className="border-border/60 bg-bg-secondary/70 relative flex shrink-0 flex-wrap items-end gap-x-4 gap-y-3 border-b px-6 py-3"
    >
      <WindowDragRegion className="absolute top-0 right-0 left-0 z-20 h-6" />

      <div className="w-[180px] shrink-0 max-[1000px]:w-[150px]">
        <AgentSelector
          agents={agents}
          selectedAgentId={selectedAgentId}
          onSelect={onAgentSelect}
          disabled={isStarting}
        />
      </div>

      <div className="shrink-0">
        <HarnessConfigControls
          agentId={selectedAgentId}
          enabled={agents.some(agent => agent.id === selectedAgentId && agent.available !== false)}
          disabled={isStarting || isLoadingSource}
          compact
        />
      </div>

      <div className="w-[180px] shrink-0 space-y-1.5 max-[1000px]:w-[150px]">
        <label className={configFieldLabelClass}>Repository</label>
        <Select.Root
          value={selectedRepoId || NO_REPOSITORY}
          onValueChange={value => onRepoSelect(value === NO_REPOSITORY ? '' : value)}
          disabled={isStarting || isLoadingSource}
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

      <button
        type="button"
        onClick={onGenerate}
        disabled={isStarting || isLoadingSource || !canGenerate}
        aria-busy={isStarting}
        className="generate-primary-button flex h-8 min-w-[132px] shrink-0 items-center justify-center gap-2 rounded-md px-3 text-xs font-medium"
      >
        {isStarting ? (
          <SpinnerGap size={14} className="animate-spin" />
        ) : (
          <Play size={13} weight="fill" />
        )}
        <span>{isStarting ? 'Starting…' : 'Start Review'}</span>
      </button>
    </div>
  );
};
