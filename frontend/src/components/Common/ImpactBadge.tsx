import React from 'react';
import { ICONS } from '../../constants/icons';

const impactConfig = {
  blocking: {
    icon: ICONS.IMPACT_BLOCKING,
    color: 'text-impact-blocking',
    bg: 'bg-impact-blocking/10',
    label: 'MUST',
  },
  nice_to_have: {
    icon: ICONS.IMPACT_NICE_TO_HAVE,
    color: 'text-impact-nice_to_have',
    bg: 'bg-impact-nice_to_have/10',
    label: 'NICE',
  },
  nitpick: {
    icon: ICONS.IMPACT_NITPICK,
    color: 'text-impact-nitpick',
    bg: 'bg-impact-nitpick/10',
    label: 'NIT',
  },
};

interface ImpactBadgeProps {
  impact: 'blocking' | 'nice_to_have' | 'nitpick';
  showIcon?: boolean;
  size?: 'sm' | 'md';
}

export const ImpactBadge: React.FC<ImpactBadgeProps> = ({
  impact,
  showIcon = true,
  size = 'sm',
}) => {
  const config = impactConfig[impact];
  const Icon = config.icon;
  const textSize = size === 'sm' ? 'text-[8px]' : 'text-[10px]';
  const iconSize = size === 'sm' ? 10 : 12;

  return (
    <span
      className={`${config.bg} ${config.color} inline-flex items-center gap-1 rounded px-1.5 py-0.5 ${textSize} font-bold`}
    >
      {showIcon && <Icon size={iconSize} weight="bold" />}
      {config.label}
    </span>
  );
};
