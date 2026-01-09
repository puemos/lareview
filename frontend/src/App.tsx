import { useEffect, useState, lazy, Suspense, useCallback } from 'react';
import { QueryClientProvider } from '@tanstack/react-query';
import { useTauri } from './hooks/useTauri';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from './store';
import { Sidebar } from './components/Layout/Sidebar';
import { createQueryClient } from './lib/query-client';
import { ErrorBoundary } from './components/Common/ErrorBoundary';
import { TooltipProvider } from './components/Common/Tooltip';

import { SettingsPageSkeleton } from './components/Settings/SettingsPageSkeleton';

const GenerateView = lazy(() =>
  import('./components/Generate/GenerateView').then(module => ({
    default: module.GenerateView,
  }))
);
const ReviewView = lazy(() =>
  import('./components/Review/ReviewView').then(module => ({
    default: module.ReviewView,
  }))
);
const SettingsView = lazy(() =>
  import('./components/Settings/SettingsView').then(module => ({
    default: module.SettingsView,
  }))
);
const ReposView = lazy(() =>
  import('./components/Repos/ReposView').then(module => ({
    default: module.ReposView,
  }))
);

const queryClient = createQueryClient();

type View = 'generate' | 'review' | 'repos' | 'settings';

function App() {
  const [currentView, setCurrentView] = useState<View>('generate');
  const [error, setError] = useState<string | null>(null);
  const { parseDiff, getPendingReviewFromState, getDiffRequest, acquireDiffFromRequest } =
    useTauri();
  const diffText = useAppStore(state => state.diffText);
  const setDiffText = useAppStore(state => state.setDiffText);
  const setParsedDiff = useAppStore(state => state.setParsedDiff);

  useEffect(() => {
    if (diffText) {
      parseDiff(diffText).then(setParsedDiff).catch(console.error);
    }
  }, [diffText, parseDiff, setParsedDiff]);

  // Unified diff loading logic for both initial load and CLI second-instance events
  const loadDiff = useCallback(async () => {
    setError(null);
    try {
      // First check if there's a pending diff (stdin, stash case)
      const pending = await getPendingReviewFromState();
      if (pending?.diff) {
        setDiffText(pending.diff);
        return;
      }

      // Then check if there's a diff request (git diff, status, pr case)
      const request = await getDiffRequest();
      if (request) {
        const result = await acquireDiffFromRequest();
        if (result.diff) {
          setDiffText(result.diff);
          // Set the review source for PR linking
          if (result.review_source) {
            useAppStore.getState().setPendingSource(result.review_source);
          }
        }
      }
    } catch (e) {
      console.error('Failed to load diff:', e);
      setError(typeof e === 'string' ? e : (e as Error).message || String(e));
    }
  }, [getPendingReviewFromState, getDiffRequest, acquireDiffFromRequest, setDiffText]);

  // Load diff on mount
  useEffect(() => {
    loadDiff();
  }, [loadDiff]);

  // Listen for CLI second-instance events and reload diff
  useEffect(() => {
    const unlisten = listen('lareview:diff-ready', () => {
      loadDiff();
    });
    return () => {
      unlisten.then(fn => fn()).catch(console.error);
    };
  }, [loadDiff]);

  const renderView = () => {
    const viewContent = (() => {
      switch (currentView) {
        case 'generate':
          return <GenerateView onNavigate={setCurrentView} />;
        case 'review':
          return (
            <ErrorBoundary>
              <ReviewView />
            </ErrorBoundary>
          );
        case 'repos':
          return <ReposView onNavigate={setCurrentView} />;
        case 'settings':
          return <SettingsView onNavigate={setCurrentView} />;
        default:
          return <GenerateView onNavigate={setCurrentView} />;
      }
    })();

    const getFallback = () => {
      if (currentView === 'settings') return <SettingsPageSkeleton />;
      return <div className="bg-bg-primary h-full" />;
    };

    return <Suspense fallback={getFallback()}>{viewContent}</Suspense>;
  };

  return (
    <QueryClientProvider client={queryClient}>
      <TooltipProvider>
        <div className="flex h-screen flex-col overflow-hidden bg-gray-900 text-white">
          {error && (
            <div className="z-50 flex items-center justify-between bg-red-500 px-4 py-2 text-white">
              <span>{error}</span>
              <button onClick={() => setError(null)} className="ml-4 rounded px-2 hover:bg-red-600">
                ✕
              </button>
            </div>
          )}
          <div className="flex flex-1 overflow-hidden">
            <Sidebar currentView={currentView} onViewChange={setCurrentView} />
            <main className="flex-1 overflow-hidden">{renderView()}</main>
          </div>
        </div>
      </TooltipProvider>
    </QueryClientProvider>
  );
}

export default App;
