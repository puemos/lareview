import React from 'react';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { motion } from 'framer-motion';
import type { DefaultIssueCategory } from '../../types';

interface CategoryBadgeProps {
  category: DefaultIssueCategory | null;
  categoryId: string;
}

export const CategoryBadge: React.FC<CategoryBadgeProps> = ({ category, categoryId }) => {
  return (
    <TooltipPrimitive.Root delayDuration={0}>
      <TooltipPrimitive.Trigger asChild>
        <span className="bg-brand/20 text-brand hover:bg-brand/30 cursor-help rounded-[2px] px-2 py-0.5 text-[9px] font-semibold tracking-wide uppercase transition-colors">
          {category?.name || categoryId}
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
                Issue Category
              </span>
              <span className="bg-brand/20 text-brand rounded-[2px] px-1.5 py-0.5 text-[9px] font-medium">
                {category?.name || categoryId}
              </span>
            </div>
            <div className="text-text-primary mb-2 text-xs leading-relaxed break-words">
              {category?.description || `Category: ${categoryId}`}
            </div>
            {category?.examples && category.examples.length > 0 && (
              <div className="border-border/30 border-t pt-1.5">
                <span className="text-text-disabled mb-1 block text-[9px] font-medium uppercase">
                  Examples:
                </span>
                <ul className="text-text-secondary space-y-0.5 text-[10px]">
                  {category.examples.slice(0, 3).map((example, idx) => (
                    <li key={idx} className="flex items-start gap-1">
                      <span className="text-text-disabled">•</span>
                      <span>{example}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
            <TooltipPrimitive.Arrow className="fill-bg-elevated" />
          </motion.div>
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
};
