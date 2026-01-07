import { useState, useEffect, useRef } from 'react';

/**
 * A hook that ensures a loading state persists for a minimum duration to prevent flashing,
 * but allows instant loading if the data is already available (cached).
 *
 * @param isLoading The actual loading state from the data source
 * @param minDuration The minimum duration in ms to show the loader if it is shown
 * @returns The effective loading state to use for the UI
 */
export function useDelayedLoading(isLoading: boolean, minDuration: number = 500) {
  const [showLoading, setShowLoading] = useState(isLoading);
  const startTime = useRef<number | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (isLoading) {
      if (!startTime.current) {
        startTime.current = Date.now();
      }
      setShowLoading(true);
      if (timerRef.current) {
        clearTimeout(timerRef.current);
        timerRef.current = null;
      }
    } else {
      if (!startTime.current) {
        // Did not start loading or was already false, so verify it is false
        setShowLoading(false);
        return;
      }

      const elapsed = Date.now() - startTime.current;
      const remaining = minDuration - elapsed;

      if (remaining > 0) {
        timerRef.current = setTimeout(() => {
          setShowLoading(false);
          startTime.current = null;
        }, remaining);
      } else {
        setShowLoading(false);
        startTime.current = null;
      }
    }

    return () => {
      if (timerRef.current) {
        clearTimeout(timerRef.current);
      }
    };
  }, [isLoading, minDuration]);

  return showLoading;
}
