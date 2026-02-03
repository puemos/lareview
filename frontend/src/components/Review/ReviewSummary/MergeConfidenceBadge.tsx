import React from 'react';
import * as TooltipPrimitive from '@radix-ui/react-tooltip';
import { motion } from 'framer-motion';
import { ICONS } from '../../../constants/icons';
import { useMergeConfidence } from '../../../hooks/useMergeConfidence';

interface MergeConfidenceBadgeProps {
  runId: string | undefined;
}

const getScoreColor = (score: number): string => {
  if (score >= 4) return 'text-status-done';
  if (score >= 3) return 'text-yellow-500';
  return 'text-impact-blocking';
};

const getScoreBgColor = (score: number): string => {
  if (score >= 4) return 'bg-status-done/10';
  if (score >= 3) return 'bg-yellow-500/10';
  return 'bg-impact-blocking/10';
};

const getBorderColor = (score: number): string => {
  if (score >= 4) return 'border-status-done/30';
  if (score >= 3) return 'border-yellow-500/30';
  return 'border-impact-blocking/30';
};

const getReasonIcon = (reason: string) => {
  if (reason.startsWith('✓') || reason.startsWith('✔')) {
    return <ICONS.ICON_CHECK size={10} className="mt-0.5 flex-shrink-0 text-status-done" weight="bold" />;
  }
  if (reason.startsWith('✗') || reason.startsWith('✘') || reason.startsWith('❌')) {
    return (
      <ICONS.ACTION_CLOSE size={10} className="mt-0.5 flex-shrink-0 text-impact-blocking" weight="bold" />
    );
  }
  if (reason.startsWith('⚠') || reason.startsWith('⚡')) {
    return (
      <ICONS.ICON_WARNING size={10} className="mt-0.5 flex-shrink-0 text-yellow-500" weight="bold" />
    );
  }
  return <ICONS.STATUS_TODO size={10} className="text-text-disabled mt-0.5 flex-shrink-0" />;
};

export const MergeConfidenceBadge: React.FC<MergeConfidenceBadgeProps> = ({ runId }) => {
  const { data, isLoading, error } = useMergeConfidence(runId);

  // Don't render anything if loading, no runId, error, or no data
  if (isLoading || !runId || error || !data) {
    return null;
  }

  const scoreColor = getScoreColor(data.score);
  const scoreBgColor = getScoreBgColor(data.score);
  const borderColor = getBorderColor(data.score);
  const hasReasons = data.reasons && data.reasons.length > 0;

  return (
    <TooltipPrimitive.Root delayDuration={0}>
      <TooltipPrimitive.Trigger asChild>
        <button
          className={`flex items-center gap-1.5 rounded-md border px-2 py-1 text-xs font-medium transition-colors hover:opacity-80 ${scoreBgColor} ${borderColor}`}
        >
          <ICONS.GAUGE size={12} className={scoreColor} />
          <span className={scoreColor}>
            {Number.isInteger(data.score) ? data.score : data.score.toFixed(1)}/5
          </span>
        </button>
      </TooltipPrimitive.Trigger>
      <TooltipPrimitive.Portal>
        <TooltipPrimitive.Content side="bottom" align="end" sideOffset={8} asChild>
          <motion.div
            initial={{ opacity: 0, scale: 0.95, y: -5 }}
            animate={{ opacity: 1, scale: 1, y: 0 }}
            exit={{ opacity: 0, scale: 0.95, y: -5 }}
            transition={{ duration: 0.15, ease: 'easeOut' }}
            className="bg-bg-elevated border-border z-50 w-72 overflow-hidden rounded-lg border shadow-xl backdrop-blur-sm"
          >
            {/* Header */}
            <div className="border-border/50 flex items-center justify-between border-b px-3 py-2">
              <div className="flex items-center gap-2">
                <ICONS.GAUGE size={14} className="text-text-secondary" />
                <span className="text-text-primary text-xs font-medium">Merge Confidence</span>
              </div>
              <div className={`flex items-center gap-1 rounded-full px-2 py-0.5 ${scoreBgColor}`}>
                <span className={`text-sm font-bold ${scoreColor}`}>
                  {Number.isInteger(data.score) ? data.score : data.score.toFixed(1)}
                </span>
                <span className={`text-xs ${scoreColor} opacity-60`}>/5</span>
              </div>
            </div>

            {/* Content */}
            <div className="p-3">
              <p className={`text-sm font-semibold ${scoreColor}`}>{data.label}</p>
              <p className="text-text-secondary mt-0.5 text-xs">{data.recommendation}</p>

              {/* Reasons */}
              {hasReasons && (
                <div className="border-border/30 mt-3 border-t pt-3">
                  <p className="text-text-disabled mb-2 text-[10px] font-medium uppercase tracking-wide">
                    Assessment
                  </p>
                  <ul className="max-h-40 space-y-1.5 overflow-y-auto">
                    {data.reasons.slice(0, 6).map((reason, i) => (
                      <li key={i} className="text-text-secondary flex items-start gap-1.5 text-xs">
                        {getReasonIcon(reason)}
                        <span className="leading-tight">{reason.replace(/^[✓✔✗✘❌⚠⚡]\s*/, '')}</span>
                      </li>
                    ))}
                    {data.reasons.length > 6 && (
                      <li className="text-text-disabled text-xs italic">
                        +{data.reasons.length - 6} more...
                      </li>
                    )}
                  </ul>
                </div>
              )}
            </div>

            <TooltipPrimitive.Arrow className="fill-bg-elevated" />
          </motion.div>
        </TooltipPrimitive.Content>
      </TooltipPrimitive.Portal>
    </TooltipPrimitive.Root>
  );
};
