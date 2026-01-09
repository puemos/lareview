import React from 'react';
import type { ReviewSource } from '../types';

export interface StartGenerationArgs {
  diffText: string;
  agentId: string;
  source?: ReviewSource | null;
}

export interface GenerationContextValue {
  startGeneration: (args: StartGenerationArgs) => Promise<boolean>;
  stopGeneration: () => Promise<void>;
}

export const GenerationContext = React.createContext<GenerationContextValue | null>(null);
