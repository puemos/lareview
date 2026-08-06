import React, { useMemo } from 'react';
import * as Select from '@radix-ui/react-select';
import { CaretDown, CaretRight, Check, Spinner } from '@phosphor-icons/react';
import clsx from 'clsx';
import { useAgentSessionConfig } from '../../hooks/useAgentSessionConfig';
import { useAppStore } from '../../store';
import { flattenConfigValues, getModelConfigCategory, isConfigGroup } from '../../lib/agent-config';
import type {
  AgentSessionConfigOption,
  AgentSessionConfigSelectGroup,
  AgentSessionConfigSelectValue,
} from '../../types';
import {
  configFieldLabelClass,
  configFieldTriggerClass,
  configSelectContentClass,
} from './configFieldStyles';

interface HarnessConfigControlsProps {
  agentId: string;
  enabled: boolean;
  disabled: boolean;
  compact?: boolean;
}

interface ConfigSelectRowProps {
  option: AgentSessionConfigOption;
  value: string;
  disabled: boolean;
  loading?: boolean;
  onChange: (value: string) => void;
  compact?: boolean;
}

const renderValueItem = (value: AgentSessionConfigSelectValue) => (
  <Select.Item
    key={value.value}
    value={value.value}
    className={clsx(
      'text-text-primary relative flex cursor-pointer items-start rounded-sm py-2 pr-9 pl-3 text-xs outline-none select-none data-[highlighted]:bg-white/10',
      'data-[disabled]:pointer-events-none data-[disabled]:opacity-50'
    )}
  >
    <div className="min-w-0 flex-1">
      <Select.ItemText>{value.name}</Select.ItemText>
      {value.description && (
        <div className="text-text-tertiary mt-0.5 line-clamp-2 text-[10px] leading-4">
          {value.description}
        </div>
      )}
    </div>
    <Select.ItemIndicator className="text-brand absolute top-2.5 right-3">
      <Check size={13} weight="bold" />
    </Select.ItemIndicator>
  </Select.Item>
);

const renderOptions = (
  options: AgentSessionConfigSelectValue[] | AgentSessionConfigSelectGroup[]
) =>
  options.map(option => {
    if (!isConfigGroup(option)) return renderValueItem(option);
    return (
      <Select.Group key={option.group}>
        <Select.Label className="text-text-disabled px-3 pt-2 pb-1 text-[9px] font-bold tracking-wider uppercase">
          {option.name}
        </Select.Label>
        {option.options.map(renderValueItem)}
      </Select.Group>
    );
  });

const ConfigSelectRow: React.FC<ConfigSelectRowProps> = ({
  option,
  value,
  disabled,
  loading = false,
  onChange,
  compact = false,
}) => {
  const values = flattenConfigValues(option);
  const selected = values.find(candidate => candidate.value === value);
  const category = getModelConfigCategory(option);
  const label =
    category === 'thought_level' ? 'Effort' : category === 'model' ? 'Model' : option.name;

  if (loading) {
    const loadingControl = (
      <div
        aria-label={`${label} updating`}
        className={clsx(
          'flex items-center rounded-md px-3 text-xs',
          compact ? 'bg-bg-tertiary border-border h-9 w-full border' : 'w-full py-2'
        )}
      >
        {!compact && <span className="text-text-secondary font-medium">{label}</span>}
        <span className="min-w-0 flex-1" />
        <span className="text-text-disabled flex items-center gap-1.5">
          <Spinner size={11} className="animate-spin" />
          <span>Updating…</span>
        </span>
      </div>
    );

    return compact ? (
      <div className="w-[170px] space-y-1.5">
        <span className={configFieldLabelClass}>{label}</span>
        {loadingControl}
      </div>
    ) : (
      loadingControl
    );
  }

  const selectControl = (
    <Select.Root value={value} onValueChange={onChange} disabled={disabled}>
      <Select.Trigger
        aria-label={label}
        className={clsx(
          'data-[state=open]:bg-white/7',
          compact
            ? `${configFieldTriggerClass} w-full`
            : 'focus-visible:ring-brand/40 flex w-full items-center rounded-md px-3 py-2 text-xs transition-colors outline-none hover:bg-white/5 focus-visible:ring-1 disabled:cursor-not-allowed disabled:opacity-60'
        )}
      >
        {!compact && <span className="text-text-secondary font-medium">{label}</span>}
        {!compact && <span className="min-w-0 flex-1" />}
        <Select.Value asChild>
          <span
            className={clsx(
              'truncate',
              compact ? 'text-text-primary max-w-[130px]' : 'text-text-tertiary max-w-[190px]'
            )}
          >
            {selected?.name || value}
          </span>
        </Select.Value>
        {compact && <span className="min-w-0 flex-1" />}
        <Select.Icon className="text-text-disabled ml-2 shrink-0">
          {compact ? <CaretDown size={12} /> : <CaretRight size={13} />}
        </Select.Icon>
      </Select.Trigger>

      <Select.Portal>
        <Select.Content
          position="popper"
          side={compact ? 'bottom' : 'left'}
          align="start"
          sideOffset={8}
          collisionPadding={12}
          className={`${configSelectContentClass} max-h-[min(420px,var(--radix-select-content-available-height))] w-[300px]`}
        >
          <Select.ScrollUpButton className="text-text-tertiary bg-bg-secondary flex h-6 items-center justify-center">
            <CaretRight size={12} className="-rotate-90" />
          </Select.ScrollUpButton>
          <Select.Viewport className="p-1">
            {option.options ? renderOptions(option.options) : null}
          </Select.Viewport>
          <Select.ScrollDownButton className="text-text-tertiary bg-bg-secondary flex h-6 items-center justify-center">
            <CaretRight size={12} className="rotate-90" />
          </Select.ScrollDownButton>
        </Select.Content>
      </Select.Portal>
    </Select.Root>
  );

  return compact ? (
    <div className="w-[170px] space-y-1.5">
      <span className={configFieldLabelClass}>{label}</span>
      {selectControl}
    </div>
  ) : (
    selectControl
  );
};

export const HarnessConfigControls: React.FC<HarnessConfigControlsProps> = ({
  agentId,
  enabled,
  disabled,
  compact = false,
}) => {
  const config = useAgentSessionConfig(agentId, enabled);
  const setPreference = useAppStore(state => state.setAgentConfigPreference);

  const visibleOptions = useMemo(
    () =>
      (config.data || []).filter(
        option => option.type === 'select' && getModelConfigCategory(option) !== null
      ),
    [config.data]
  );

  if (!enabled || (!config.isPending && !config.error && visibleOptions.length === 0)) {
    return null;
  }

  if (config.error && visibleOptions.length === 0) {
    const errorControl = (
      <button
        type="button"
        onClick={() => config.refetch()}
        disabled={disabled || config.isFetching}
        title={config.error instanceof Error ? config.error.message : String(config.error)}
        className={clsx(
          'text-text-disabled hover:text-text-secondary flex items-center justify-between text-[10px] transition-colors disabled:opacity-60',
          compact ? 'bg-bg-tertiary border-border h-9 w-full rounded-md border px-3' : 'w-full px-1'
        )}
      >
        <span>Using harness defaults</span>
        <span>Retry options</span>
      </button>
    );

    return compact ? (
      <div className="w-[170px] space-y-1.5">
        <span className={configFieldLabelClass}>Configuration</span>
        {errorControl}
      </div>
    ) : (
      errorControl
    );
  }

  if (config.isPending && visibleOptions.length === 0) {
    const loadingControl = (
      <div
        className={clsx(
          'text-text-disabled flex items-center gap-2 text-[10px]',
          compact ? 'bg-bg-tertiary border-border h-9 w-full rounded-md border px-3' : 'px-1'
        )}
      >
        <Spinner size={11} className="animate-spin" />
        <span>Loading harness options…</span>
      </div>
    );

    return compact ? (
      <div className="w-[170px] space-y-1.5">
        <span className={configFieldLabelClass}>Configuration</span>
        {loadingControl}
      </div>
    ) : (
      loadingControl
    );
  }

  return (
    <div
      className={clsx(
        compact
          ? 'flex items-end gap-3'
          : 'border-border/70 bg-bg-tertiary/45 rounded-lg border p-1'
      )}
    >
      {visibleOptions.map(option => {
        const category = getModelConfigCategory(option);
        const preference = config.preferences.find(item => item.configId === option.id);
        const preferredValue = preference?.value;
        const value =
          typeof preferredValue === 'string' &&
          flattenConfigValues(option).some(candidate => candidate.value === preferredValue)
            ? preferredValue
            : String(option.currentValue);

        return (
          <ConfigSelectRow
            key={option.id}
            option={option}
            value={value}
            disabled={disabled}
            loading={config.isFetching && category === 'thought_level'}
            compact={compact}
            onChange={nextValue =>
              setPreference(
                agentId,
                { configId: option.id, value: nextValue, category: category || undefined },
                category === 'model' ? ['thought_level'] : []
              )
            }
          />
        );
      })}
    </div>
  );
};
