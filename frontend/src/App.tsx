import { useEffect, useState, lazy, Suspense, useCallback } from 'react';
import { Toaster, toast } from 'sonner';

import { QueryClientProvider } from '@tanstack/react-query';
import { useTauri } from './hooks/useTauri';
import { listen } from '@tauri-apps/api/event';
import { useAppStore } from './store';
import { Sidebar } from './components/Layout/Sidebar';
import { createQueryClient } from './lib/query-client';
import { ErrorBoundary } from './components/Common/ErrorBoundary';
import { TooltipProvider } from './components/Common/Tooltip';
import { ICONS } from './constants/icons';
import { GenerationProvider } from './contexts/GenerationContext';
import { useUpdateCheck } from './hooks/useUpdateCheck';
import { UpdateModal } from './components/Common/UpdateModal';

import { SettingsPageSkeleton } from './components/Settings/SettingsPageSkeleton';
import { ReviewViewSkeleton } from './components/Review/ReviewViewSkeleton';

const PageSkeleton = () => (
  <div className="bg-bg-primary flex h-full animate-pulse items-center justify-center">
    <div className="bg-bg-tertiary h-8 w-8 rounded-full" />
  </div>
);

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
const RulesView = lazy(() =>
  import('./components/Rules/RulesView').then(module => ({
    default: module.RulesView,
  }))
);
const LearningView = lazy(() =>
  import('./components/Learning/LearningView').then(module => ({
    default: module.LearningView,
  }))
);

const queryClient = createQueryClient();

type View = 'generate' | 'review' | 'repos' | 'rules' | 'learning' | 'settings';

function App() {
  const [currentView, setCurrentView] = useState<View>('generate');
  const [error, setError] = useState<string | null>(null);
  const [showUpdateModal, setShowUpdateModal] = useState(false);
  const { parseDiff, getPendingReviewFromState, getDiffRequest, acquireDiffFromRequest } =
    useTauri();
  const { currentVersion, updateAvailable } = useUpdateCheck();
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

  useEffect(() => {
    if (updateAvailable) {
      toast('Update Available', {
        description: `LaReview v${updateAvailable.latestVersion} is available. You're on v${currentVersion}.`,
        duration: 10000,
        action: {
          label: 'Details',
          onClick: () => setShowUpdateModal(true),
        },
      });
    }
  }, [updateAvailable, currentVersion]);

  const reviewViewMode = useAppStore(state => state.reviewViewMode);

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
        case 'rules':
          return <RulesView />;
        case 'learning':
          return <LearningView />;
        case 'settings':
          return <SettingsView onNavigate={setCurrentView} />;
        default:
          return <GenerateView onNavigate={setCurrentView} />;
      }
    })();

    const getFallback = () => {
      switch (currentView) {
        case 'settings':
          return <SettingsPageSkeleton />;
        case 'review':
          return <ReviewViewSkeleton mode={reviewViewMode} />;
        default:
          return <PageSkeleton />;
      }
    };

    return <Suspense fallback={getFallback()}>{viewContent}</Suspense>;
  };

  return (
    <QueryClientProvider client={queryClient}>
      <GenerationProvider>
        <TooltipProvider>
          <div className="flex h-screen flex-col overflow-hidden bg-gray-900 text-white">
            <Toaster
              position="bottom-right"
              theme="dark"
              closeButton
              icons={{
                success: <ICONS.STATUS_DONE size={18} className="text-green-400" />,
                info: <ICONS.ICON_INFO size={18} className="text-blue-400" />,
                warning: <ICONS.ICON_WARNING size={18} className="text-yellow-400" />,
                error: <ICONS.STATUS_IGNORED size={18} className="text-red-400" />,
                loading: <ICONS.ACTION_LOADING size={18} className="text-brand animate-spin" />,
              }}
              toastOptions={{
                classNames: {
                  title: '!font-semibold',
                  description: '!text-text-secondary !text-[11px]',
                },
                className:
                  '!shadow-custom !bg-bg-tertiary !text-text-primary !rounded-lg !border !border-border !py-3',
              }}
            />

            {error && (
              <div className="z-50 flex items-center justify-between bg-red-500 px-4 py-2 text-white">
                <span>{error}</span>
                <button
                  onClick={() => setError(null)}
                  className="ml-4 rounded px-2 hover:bg-red-600"
                >
                  ✕
                </button>
              </div>
            )}
            <div className="flex flex-1 overflow-hidden">
              <Sidebar
                currentView={currentView}
                onViewChange={setCurrentView}
                currentVersion={currentVersion}
                updateAvailable={updateAvailable}
                onUpdateClick={() => setShowUpdateModal(true)}
              />
              <main className="flex-1 overflow-hidden">{renderView()}</main>
            </div>
          </div>
          {updateAvailable && currentVersion && (
            <UpdateModal
              isOpen={showUpdateModal}
              onClose={() => setShowUpdateModal(false)}
              currentVersion={currentVersion}
              updateInfo={updateAvailable}
            />
          )}
        </TooltipProvider>
      </GenerationProvider>
    </QueryClientProvider>
  );
}

export default App;
