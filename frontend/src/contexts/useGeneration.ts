import React from 'react';
import { GenerationContext, type GenerationContextValue } from './generation-context';

export const useGeneration = (): GenerationContextValue => {
  const ctx = React.useContext(GenerationContext);
  if (!ctx) {
    throw new Error('useGeneration must be used within GenerationProvider');
  }
  return ctx;
};
