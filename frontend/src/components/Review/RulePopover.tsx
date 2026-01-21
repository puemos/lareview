import React from 'react';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { motion } from 'framer-motion';
import { ReviewRule } from '../../types';

interface RulePopoverProps {
  rule: ReviewRule | null;
  ruleId: string;
}

export const RulePopover: React.FC<RulePopoverProps> = ({ rule, ruleId }) => {
  return (
    <TooltipPrimitive.Root delayDuration={0}>
      <TooltipPrimitive.Trigger asChild>
        <span className="bg-bg-tertiary text-text-tertiary hover:bg-bg-tertiary/70 cursor-help rounded-[2px] px-2 py-0.5 text-[9px] font-semibold tracking-wide uppercase transition-colors">
          Rule
        </span>
      </TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content side="top" align="end" sideOffset={8} asChild>
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: 5 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: 5 }}
            transition={{ duration: 0.15, ease: 'easeOut' }}
            className="bg-bg-elevated border-border z-50 max-w-xs overflow-hidden rounded-[2px] border p-3 shadow-xl backdrop-blur-sm"
          >
            <div className="border-border/50 mb-2 flex items-center justify-between border-b pb-1.5 focus:outline-none">
              <span className="text-text-tertiary text-[10px] font-bold tracking-tight uppercase">
                Rule Details
              </span>
              <span className="bg-bg-tertiary text-text-tertiary rounded-[2px] px-1.5 py-0.5 text-[9px] font-medium">
                {rule?.scope || 'Global'}
              </span>
            </div>
            <div className="text-text-primary mb-2 text-xs leading-relaxed break-words">
              {rule?.text || `Rule ID: ${ruleId}`}
            </div>
            {rule?.glob && (
              <div className="border-border/30 flex items-center gap-1.5 border-t pt-1.5">
                <span className="text-text-disabled min-w-[32px] text-[9px] font-medium uppercase">
                  Scope:
                </span>
                <code className="bg-bg-tertiary text-text-secondary rounded-[2px] px-1 py-0.5 font-mono text-[9px]">
                  {rule.glob}
                </code>
              </div>
            )}
            <TooltipPrimitive.Arrow className="fill-bg-elevated" />
          </motion.div>
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
};
