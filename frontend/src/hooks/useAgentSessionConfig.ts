import { useEffect, useMemo } from 'react';
import { useQuery } from '@tanstack/react-query';
import { useTauri } from './useTauri';
import { useAppStore } from '../store';
import type { AgentConfigPreference } from '../types';
import { isPreferenceSupported, toAgentConfigSelections } from '../lib/agent-config';

const EMPTY_PREFERENCES: AgentConfigPreference[] = [];

export function useAgentSessionConfig(agentId: string, enabled: boolean) {
  const { getAgentSessionConfig } = useTauri();
  const preferences = useAppStore(
    state => state.agentConfigPreferences[agentId] || EMPTY_PREFERENCES
  );
  const setAgentConfigPreferences = useAppStore(state => state.setAgentConfigPreferences);
  const selections = useMemo(() => toAgentConfigSelections(preferences), [preferences]);

  const query = useQuery({
    queryKey: ['agent-session-config', agentId, selections],
    queryFn: () => getAgentSessionConfig(agentId, selections),
    enabled: enabled && !!agentId,
    retry: false,
    staleTime: Infinity,
    refetchOnWindowFocus: false,
    placeholderData: previousData => previousData,
  });

  useEffect(() => {
    if (!query.isSuccess || query.isPlaceholderData) return;
    const valid = preferences.filter(preference => isPreferenceSupported(query.data, preference));
    if (valid.length !== preferences.length) {
      setAgentConfigPreferences(agentId, valid);
    }
  }, [
    agentId,
    preferences,
    query.data,
    query.isPlaceholderData,
    query.isSuccess,
    setAgentConfigPreferences,
  ]);

  return {
    ...query,
    preferences,
    selections,
  };
}
