import type {
  AgentConfigPreference,
  AgentConfigSelection,
  AgentSessionConfigOption,
  AgentSessionConfigSelectGroup,
  AgentSessionConfigSelectValue,
} from '../types';

export const isConfigGroup = (
  value: AgentSessionConfigSelectValue | AgentSessionConfigSelectGroup
): value is AgentSessionConfigSelectGroup => 'group' in value;

export const flattenConfigValues = (
  option: AgentSessionConfigOption
): AgentSessionConfigSelectValue[] => {
  if (option.type !== 'select' || !option.options) return [];
  return option.options.flatMap(value => (isConfigGroup(value) ? value.options : [value]));
};

export const getModelConfigCategory = (
  option: AgentSessionConfigOption
): 'model' | 'thought_level' | null => {
  if (option.category === 'model' || option.category === 'thought_level') {
    return option.category;
  }

  // Categories are optional in ACP. Use labels only as a presentation fallback; the option ID and
  // advertised values remain the source of truth sent back to the harness.
  const hint = `${option.id} ${option.name}`.toLowerCase();
  if (hint.includes('model')) return 'model';
  if (hint.includes('reason') || hint.includes('effort') || hint.includes('thought')) {
    return 'thought_level';
  }
  return null;
};

export const isPreferenceSupported = (
  options: AgentSessionConfigOption[],
  preference: AgentConfigPreference
): boolean => {
  const option = options.find(candidate => candidate.id === preference.configId);
  if (!option) return false;
  if (option.type === 'boolean') return typeof preference.value === 'boolean';
  if (typeof preference.value !== 'string') return false;
  return flattenConfigValues(option).some(value => value.value === preference.value);
};

const categoryPriority = (category?: string): number => {
  if (category === 'model') return 0;
  if (category === 'thought_level') return 1;
  return 2;
};

export const toAgentConfigSelections = (
  preferences: AgentConfigPreference[]
): AgentConfigSelection[] =>
  preferences
    .map((preference, index) => ({ preference, index }))
    .sort(
      (left, right) =>
        categoryPriority(left.preference.category) - categoryPriority(right.preference.category) ||
        left.index - right.index
    )
    .map(({ preference }) => ({
      configId: preference.configId,
      value: preference.value,
    }));
