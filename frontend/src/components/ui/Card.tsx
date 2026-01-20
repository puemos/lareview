import clsx from 'clsx';

interface CardProps {
  children: React.ReactNode;
  className?: string;
  style?: React.CSSProperties;
}

export const Card: React.FC<CardProps> = ({ children, className, style }) => (
  <div
    className={clsx(
      "group bg-bg-secondary/40 hover:bg-bg-secondary hover:border-border rounded-lg border border-transparent transition-all",
      className
    )}
    style={style}
  >
    {children}
  </div>
);
