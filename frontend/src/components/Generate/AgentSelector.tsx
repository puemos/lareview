import React from 'react';
import * as Select from '@radix-ui/react-select';
import { Check, CaretDown, User } from '@phosphor-icons/react';
import clsx from 'clsx';

// Import Icons
import geminiIcon from '../../assets/icons/gemini.svg';
import claudeIcon from '../../assets/icons/claude.svg';
import mistralIcon from '../../assets/icons/mistral.svg';
import kimiIcon from '../../assets/icons/kimi.svg';
import qwenIcon from '../../assets/icons/qwen.svg';

import grokIcon from '../../assets/icons/grok.svg';
import opencodeIcon from '../../assets/icons/opencode.svg';
import codexIcon from '../../assets/icons/codex.svg';

const AGENT_ICONS: Record<string, string> = {
  gemini: geminiIcon,
  claude: claudeIcon,
  mistral: mistralIcon,
  kimi: kimiIcon,
  qwen: qwenIcon,

  grok: grokIcon,
  opencode: opencodeIcon,
  codex: codexIcon,
};

interface Agent {
  id: string;
  name: string;
  available?: boolean;
}

interface AgentSelectorProps {
  agents: Agent[];
  selectedAgentId: string;
  onSelect: (id: string) => void;
}

export const AgentSelector: React.FC<AgentSelectorProps> = ({
  agents,
  selectedAgentId,
  onSelect,
}) => {
  const selectedAgent = agents.find(a => a.id === selectedAgentId);

  return (
    <div className="space-y-1.5">
      <label className="text-text-disabled flex items-center gap-1.5 text-[10px] font-bold tracking-wider uppercase">
        <User size={12} /> Agent
      </label>
      <Select.Root value={selectedAgentId} onValueChange={onSelect}>
        <Select.Trigger
          className="bg-bg-tertiary border-border text-text-primary focus:border-brand focus:ring-brand/20 hover:border-text-disabled data-[placeholder]:text-text-disabled inline-flex w-full items-center justify-between rounded-md border px-3 py-2 text-xs transition-all focus:ring-1 focus:outline-none"
          aria-label="Agent"
        >
          <Select.Value asChild>
            <div className="flex items-center gap-2">
              {selectedAgent && (
                <img
                  src={AGENT_ICONS[selectedAgent.id] || AGENT_ICONS.codex /* fallback */}
                  alt=""
                  className="h-4 w-4 object-contain"
                />
              )}
              <span className="truncate">{selectedAgent?.name || 'Select an agent...'}</span>
            </div>
          </Select.Value>
          <Select.Icon className="text-text-disabled">
            <CaretDown size={12} />
          </Select.Icon>
        </Select.Trigger>

        <Select.Portal>
          <Select.Content
            className="bg-bg-secondary border-border z-50 min-w-[var(--radix-select-trigger-width)] overflow-hidden rounded-md border shadow-lg"
            position="popper"
            sideOffset={5}
          >
            <Select.Viewport className="p-1">
              {agents.map(agent => (
                <Select.Item
                  key={agent.id}
                  value={agent.id}
                  disabled={agent.available === false}
                  className={clsx(
                    'text-text-primary relative flex cursor-pointer items-center gap-2 rounded-sm py-2 pr-8 pl-2 text-xs outline-none select-none',
                    'data-[highlighted]:text-text-primary data-[highlighted]:bg-white/10',
                    'data-[disabled]:pointer-events-none data-[disabled]:opacity-50'
                  )}
                >
                  <img
                    src={AGENT_ICONS[agent.id] || AGENT_ICONS.codex}
                    alt=""
                    className="h-4 w-4 object-contain"
                  />
                  <div className="flex items-center gap-2">
                    <Select.ItemText>{agent.name}</Select.ItemText>
                    {agent.available === false && (
                      <span className="text-[10px] font-medium text-red-400">(Unavailable)</span>
                    )}
                  </div>
                  <div className="absolute right-2 flex w-4 items-center justify-center">
                    <Select.ItemIndicator>
                      <Check size={12} />
                    </Select.ItemIndicator>
                  </div>
                </Select.Item>
              ))}
            </Select.Viewport>
          </Select.Content>
        </Select.Portal>
      </Select.Root>
    </div>
  );
};
