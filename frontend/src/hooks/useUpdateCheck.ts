import { useEffect, useState } from 'react';
import { useTauri } from './useTauri';
import { compareVersions, fetchLatestRelease } from '../lib/version';

export interface UpdateInfo {
  latestVersion: string;
  releaseUrl: string;
  releaseName: string;
  releaseNotes: string;
}

export function useUpdateCheck() {
  const { getVersion } = useTauri();
  const [currentVersion, setCurrentVersion] = useState<string | null>(null);
  const [updateAvailable, setUpdateAvailable] = useState<UpdateInfo | null>(null);

  useEffect(() => {
    let cancelled = false;

    async function check() {
      try {
        const [version, release] = await Promise.all([getVersion(), fetchLatestRelease()]);
        if (cancelled) return;

        setCurrentVersion(version);

        if (release && compareVersions(version, release.tag_name) < 0) {
          setUpdateAvailable({
            latestVersion: release.tag_name.replace(/^v/, ''),
            releaseUrl: release.html_url,
            releaseName: release.name,
            releaseNotes: release.body,
          });
        }
      } catch {
        // Fail silently — version display will just stay null
      }
    }

    check();
    return () => {
      cancelled = true;
    };
  }, [getVersion]);

  return { currentVersion, updateAvailable };
}
