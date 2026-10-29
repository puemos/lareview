import type { ReactNode } from 'react';

interface EmptyPanelProps {
  icon?: ReactNode;
  title: string;
  description?: string;
  action?: ReactNode;
}

export function EmptyPanel({ icon, title, description, action }: EmptyPanelProps) {
  return (
    <div className="product-empty">
      {icon && (
        <div className="product-empty-icon" aria-hidden="true">
          {icon}
        </div>
      )}
      <h2>{title}</h2>
      {description && <p>{description}</p>}
      {action && <div className="mt-5">{action}</div>}
    </div>
  );
}
