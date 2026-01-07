import { Component, ErrorInfo, ReactNode } from 'react';
import { ICONS } from '../../constants/icons';

interface Props {
  children: ReactNode;
  fallback?: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export class ErrorBoundary extends Component<Props, State> {
  public state: State = {
    hasError: false,
    error: null,
  };

  public static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  public componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('ErrorBoundary caught an error:', error, errorInfo);
  }

  public render() {
    if (this.state.hasError) {
      if (this.props.fallback) {
        return this.props.fallback;
      }

      return (
        <div className="bg-bg-primary text-text-disabled flex h-full items-center justify-center p-8">
          <div className="max-w-md text-center">
            <div className="bg-status-ignored/10 mx-auto mb-4 flex h-16 w-16 items-center justify-center rounded-2xl">
              <ICONS.ICON_WARNING size={32} className="text-status-ignored" />
            </div>
            <h2 className="text-text-primary mb-2 text-lg font-medium">Something went wrong</h2>
            <p className="text-text-secondary mb-4 text-sm">
              {this.state.error?.message || 'An unexpected error occurred'}
            </p>
            <button
              onClick={() => {
                this.setState({ hasError: false, error: null });
                window.location.reload();
              }}
              className="bg-brand text-bg-primary rounded-md px-4 py-2 text-sm font-medium transition-all hover:brightness-110"
            >
              Reload Page
            </button>
          </div>
        </div>
      );
    }

    return this.props.children;
  }
}

export function createErrorBoundary(fallback?: ReactNode) {
  return function ErrorBoundaryWrapper({ children }: { children: ReactNode }) {
    return <ErrorBoundary fallback={fallback}>{children}</ErrorBoundary>;
  };
}
