import { describe, it, expect, vi, beforeEach } from 'vitest';
import { compareVersions, fetchLatestRelease } from '../version';

describe('compareVersions', () => {
  it('returns 0 for equal versions', () => {
    expect(compareVersions('1.2.3', '1.2.3')).toBe(0);
  });

  it('returns -1 when a < b', () => {
    expect(compareVersions('0.0.31', '0.0.32')).toBe(-1);
  });

  it('returns 1 when a > b', () => {
    expect(compareVersions('0.1.0', '0.0.32')).toBe(1);
  });

  it('strips v prefix', () => {
    expect(compareVersions('v0.0.32', '0.0.32')).toBe(0);
    expect(compareVersions('0.0.31', 'v0.0.32')).toBe(-1);
  });

  it('handles different length versions', () => {
    expect(compareVersions('1.0', '1.0.0')).toBe(0);
    expect(compareVersions('1.0', '1.0.1')).toBe(-1);
  });

  it('handles major version differences', () => {
    expect(compareVersions('1.0.0', '2.0.0')).toBe(-1);
    expect(compareVersions('2.0.0', '1.9.9')).toBe(1);
  });
});

describe('fetchLatestRelease', () => {
  beforeEach(() => {
    vi.restoreAllMocks();
  });

  it('returns release info on success', async () => {
    const mockResponse = {
      tag_name: 'v0.0.33',
      html_url: 'https://github.com/puemos/lareview/releases/tag/v0.0.33',
      name: 'v0.0.33',
      body: '## Bug Fixes\n- Fixed a thing',
    };

    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: true,
      json: () => Promise.resolve(mockResponse),
    } as Response);

    const result = await fetchLatestRelease();

    expect(result).toEqual({
      tag_name: 'v0.0.33',
      html_url: 'https://github.com/puemos/lareview/releases/tag/v0.0.33',
      name: 'v0.0.33',
      body: '## Bug Fixes\n- Fixed a thing',
    });
  });

  it('returns null on non-ok response', async () => {
    vi.spyOn(globalThis, 'fetch').mockResolvedValue({
      ok: false,
      status: 404,
    } as Response);

    const result = await fetchLatestRelease();
    expect(result).toBeNull();
  });

  it('returns null on network error', async () => {
    vi.spyOn(globalThis, 'fetch').mockRejectedValue(new Error('Network error'));

    const result = await fetchLatestRelease();
    expect(result).toBeNull();
  });
});
