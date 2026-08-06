import { describe, expect, it } from 'vitest';
import {
  flattenConfigValues,
  getModelConfigCategory,
  isPreferenceSupported,
  toAgentConfigSelections,
} from '../agent-config';
import type { AgentSessionConfigOption } from '../../types';

const modelOption: AgentSessionConfigOption = {
  id: 'chosen_model',
  name: 'Model',
  category: 'model',
  type: 'select',
  currentValue: 'model-a',
  options: [
    { value: 'model-a', name: 'Model A' },
    { value: 'model-b', name: 'Model B' },
  ],
};

const effortOption: AgentSessionConfigOption = {
  id: 'reasoning_effort',
  name: 'Reasoning effort',
  category: 'thought_level',
  type: 'select',
  currentValue: 'high',
  options: [
    {
      group: 'reasoning',
      name: 'Reasoning',
      options: [
        { value: 'low', name: 'Low' },
        { value: 'high', name: 'High' },
      ],
    },
  ],
};

describe('agent config helpers', () => {
  it('flattens grouped harness values without changing their identifiers', () => {
    expect(flattenConfigValues(effortOption).map(value => value.value)).toEqual(['low', 'high']);
  });

  it('applies model preferences before dependent effort preferences', () => {
    expect(
      toAgentConfigSelections([
        { configId: 'reasoning_effort', value: 'high', category: 'thought_level' },
        { configId: 'chosen_model', value: 'model-b', category: 'model' },
      ])
    ).toEqual([
      { configId: 'chosen_model', value: 'model-b' },
      { configId: 'reasoning_effort', value: 'high' },
    ]);
  });

  it('rejects a persisted value removed by the harness', () => {
    expect(
      isPreferenceSupported([modelOption, effortOption], {
        configId: 'reasoning_effort',
        value: 'ultra',
        category: 'thought_level',
      })
    ).toBe(false);
  });

  it('recognizes uncategorized model and effort options as a UI fallback', () => {
    expect(getModelConfigCategory({ ...modelOption, category: undefined })).toBe('model');
    expect(getModelConfigCategory({ ...effortOption, category: undefined })).toBe('thought_level');
  });
});
