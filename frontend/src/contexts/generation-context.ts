import React from 'react';
import type { AgentConfigSelection, ReviewSource } from '../types';

export interface StartGenerationArgs {
  diffText: string;
  agentId: string;
  repoId?: string;
  source?: ReviewSource | null;
  agentConfig?: AgentConfigSelection[];
}

export interface GenerationContextValue {
  startGeneration: (args: StartGenerationArgs) => Promise<boolean>;
  stopGeneration: () => Promise<void>;
}

export const GenerationContext = React.createContext<GenerationContextValue | null>(null);
