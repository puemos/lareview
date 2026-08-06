import React from 'react';
import * as Select from '@radix-ui/react-select';
import { Check, CaretDown, Robot } from '@phosphor-icons/react';
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
import {
  configFieldLabelClass,
  configFieldTriggerClass,
  configSelectContentClass,
  configSelectItemClass,
} from './configFieldStyles';

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
      <label className={configFieldLabelClass}>Agent</label>
      <Select.Root value={selectedAgentId} onValueChange={onSelect}>
        <Select.Trigger
          className={`${configFieldTriggerClass} data-[placeholder]:text-text-disabled w-full justify-between`}
          aria-label="Agent"
        >
          <Select.Value asChild>
            <div className="flex items-center gap-2">
              {selectedAgent &&
                (AGENT_ICONS[selectedAgent.id] ? (
                  <img
                    src={AGENT_ICONS[selectedAgent.id]}
                    alt=""
                    className="h-4 w-4 object-contain"
                  />
                ) : (
                  <Robot size={16} weight="fill" className="text-brand" />
                ))}
              <span className="truncate">{selectedAgent?.name || 'Select an agent...'}</span>
            </div>
          </Select.Value>
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
              {agents.map(agent => (
                <Select.Item
                  key={agent.id}
                  value={agent.id}
                  disabled={agent.available === false}
                  className={clsx(configSelectItemClass, 'gap-2')}
                >
                  {AGENT_ICONS[agent.id] ? (
                    <img src={AGENT_ICONS[agent.id]} alt="" className="h-4 w-4 object-contain" />
                  ) : (
                    <Robot size={16} weight="fill" className="text-brand" />
                  )}
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
