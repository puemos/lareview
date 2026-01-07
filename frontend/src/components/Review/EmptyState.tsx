import React from 'react';
import { ICONS } from '../../constants/icons';

interface EmptyStateProps {
  title: string;
  description: string;
}

export const EmptyState: React.FC<EmptyStateProps> = ({ title, description }) => {
  return (
    <div className="bg-bg-primary text-text-disabled flex h-full flex-col items-center justify-center">
      <div className="space-y-4 text-center opacity-50">
        <div className="bg-bg-secondary mx-auto flex h-16 w-16 items-center justify-center rounded-2xl shadow-sm">
          <ICONS.ICON_PLAN size={32} />
        </div>
        <div>
          <h2 className="text-text-primary mb-1 text-sm font-medium">{title}</h2>
          <p className="text-text-tertiary text-xs">{description}</p>
        </div>
      </div>
    </div>
  );
};
