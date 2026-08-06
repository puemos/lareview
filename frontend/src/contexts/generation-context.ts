import React from 'react';
import type { AgentConfigSelection, ReviewRunStatus, ReviewSource } from '../types';

export interface StartGenerationArgs {
  diffText: string;
  agentId: string;
  repoId?: string;
  source?: ReviewSource | null;
  agentConfig?: AgentConfigSelection[];
}

export interface GenerationContextValue {
  startGeneration: (args: StartGenerationArgs) => Promise<{
    review_id: string;
    run_id: string;
    status: ReviewRunStatus;
  } | null>;
  stopGeneration: (runId: string) => Promise<void>;
}

export const GenerationContext = React.createContext<GenerationContextValue | null>(null);
