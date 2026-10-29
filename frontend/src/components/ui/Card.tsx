import clsx from 'clsx';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export const Card: React.FC<CardProps> = ({ children, className, style }) => (
  <div
    className={clsx(
      'group bg-bg-secondary/20 border-border/50 rounded-lg border transition-colors',
      className
    )}
    style={style}
  >
    {children}
  </div>
);
