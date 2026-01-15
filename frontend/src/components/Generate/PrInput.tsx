import React, { useMemo } from 'react';
import { GitMerge, GithubLogo, GitlabLogo, Spinner } from '@phosphor-icons/react';
import { AnimatePresence, motion } from 'framer-motion';

interface PrInputProps {
  prRef: string;
  onPrRefChange: (value: string) => void;
  onFetch: () => void;
  isLoading: boolean;
  disabled: boolean;
}

export const PrInput: React.FC<PrInputProps> = ({
  prRef,
  onPrRefChange,
  onFetch,
  isLoading,
  disabled,
}) => {
  const detectedProvider = useMemo(() => {
    const value = prRef.trim();
    if (!value) return 'generic';

    const githubUrl = /https?:\/\/github\.com\/[^/\s]+\/[^/\s]+\/pull\/\d+/i;
    const githubShort = /^[^/\s]+\/[^/\s]+#\d+$/i;
    const githubLegacy = /^[^/\s]+\/[^/\s]+\/\d+$/i;
    const gitlabUrl = /https?:\/\/[^/\s]+\/.+\/-\/merge_requests\/\d+/i;
    const gitlabShort = /^[^\s!#]+(?:\/[^\s!#]+)*!\d+$/i;

    if (githubUrl.test(value) || githubShort.test(value) || githubLegacy.test(value)) {
      return 'github';
    }
    if (gitlabUrl.test(value) || gitlabShort.test(value)) {
      return 'gitlab';
    }
    return 'generic';
  }, [prRef]);

  const Icon =
    detectedProvider === 'github'
      ? GithubLogo
      : detectedProvider === 'gitlab'
        ? GitlabLogo
        : GitMerge;
  const iconKey = detectedProvider;

  return (
    <>
      <div className="text-text-disabled relative flex flex-shrink-0 items-center border-r border-white/5 px-2">
        <AnimatePresence mode="wait">
          <motion.span
            key={iconKey}
            initial={{ opacity: 0, y: -6 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 6 }}
            transition={{ duration: 0.18, ease: 'easeOut' }}
          >
            <Icon size={14} />
          </motion.span>
        </AnimatePresence>
      </div>
      <input
        type="text"
        value={prRef}
        onChange={e => onPrRefChange(e.target.value)}
        placeholder="Paste a remote link or shorthand..."
        className="text-text-primary placeholder-text-disabled min-w-0 flex-1 bg-transparent px-3 py-1.5 font-mono text-xs focus:outline-none"
        disabled={isLoading}
      />
      <div className="flex h-[14px] flex-shrink-0 items-center border-l border-white/5"></div>
      <button
        onClick={onFetch}
        disabled={!prRef.trim() || isLoading || disabled}
        className="flex min-w-[48px] flex-shrink-0 items-center justify-center gap-1 px-3 py-1.5 text-[10px] font-medium opacity-80 transition-colors hover:opacity-100 disabled:opacity-50"
      >
        {isLoading ? <Spinner size={14} className="text-text-primary animate-spin" /> : 'Fetch'}
      </button>
    </>
  );
};
