export interface ReleaseInfo {
  tag_name: string;
  html_url: string;
  name: string;
  body: string;
}

/**
 * Compare two semver version strings.
 * Returns -1 if a < b, 0 if equal, 1 if a > b.
 */
export function compareVersions(a: string, b: string): number {
  const normalize = (v: string) => v.replace(/^v/, '');
  const partsA = normalize(a).split('.').map(Number);
  const partsB = normalize(b).split('.').map(Number);

  for (let i = 0; i < Math.max(partsA.length, partsB.length); i++) {
    const numA = partsA[i] ?? 0;
    const numB = partsB[i] ?? 0;
    if (numA < numB) return -1;
    if (numA > numB) return 1;
  }
  return 0;
}

export async function fetchLatestRelease(): Promise<ReleaseInfo | null> {
  try {
    const res = await fetch(
      'https://api.github.com/repos/puemos/lareview/releases/latest',
      {
        headers: { Accept: 'application/vnd.github.v3+json' },
      }
    );
    if (!res.ok) return null;
    const data = await res.json();
    return {
      tag_name: data.tag_name,
      html_url: data.html_url,
      name: data.name,
      body: data.body ?? '',
    };
  } catch {
    return null;
  }
}
