import { useState, useEffect, useMemo, useCallback } from 'react';
import { parsePatchFiles } from '@pierre/diffs';
import { FileDiff } from '@pierre/diffs/react';
import type { FileDiffMetadata, FileDiffOptions } from '@pierre/diffs';
import {
  GitPullRequest,
  GitCommit,
  ExternalLink,
  Share2,
  Check,
  Search,
  AlertCircle,
  Loader2,
  FileCode2,
  Key,
  Copy,
  Sparkles,
  RefreshCw,
  FolderOpen,
} from 'lucide-react';
import { useSettings } from './SettingsContext';
import { settingsToFileDiffOptions } from '../settingsTypes';
import { AiInsightsDrawer } from './AiInsightsDrawer';


export interface GitHubTarget {
  owner: string;
  repo: string;
  type: 'pr' | 'commit';
  id: string;
}

export function parseGitHubTarget(input: string): GitHubTarget | null {
  const trimmed = input.trim();
  if (!trimmed) return null;

  // Format: owner/repo#number (e.g. facebook/react#28000)
  const hashMatch = trimmed.match(/^([a-zA-Z0-9_.-]+)\/([a-zA-Z0-9_.-]+)#(\d+)$/);
  if (hashMatch) {
    return { owner: hashMatch[1], repo: hashMatch[2], type: 'pr', id: hashMatch[3] };
  }

  // Format: owner/repo@sha (e.g. chid/diff-tool@0afbd12)
  const atMatch = trimmed.match(/^([a-zA-Z0-9_.-]+)\/([a-zA-Z0-9_.-]+)@([a-f0-9]+)$/i);
  if (atMatch) {
    return { owner: atMatch[1], repo: atMatch[2], type: 'commit', id: atMatch[3] };
  }

  try {
    const url = new URL(trimmed.startsWith('http') ? trimmed : `https://${trimmed}`);
    const parts = url.pathname.replace(/^\//, '').split('/');
    if (parts.length >= 4) {
      const owner = parts[0];
      const repo = parts[1];
      const kind = parts[2];
      const id = parts[3];
      if (kind === 'pull' || kind === 'pulls') {
        return { owner, repo, type: 'pr', id };
      }
      if (kind === 'commit' || kind === 'commits') {
        return { owner, repo, type: 'commit', id };
      }
    }
  } catch {
    // invalid URL format
  }

  return null;
}

interface ReviewItemInfo {
  title: string;
  numberOrSha: string;
  author: string;
  authorAvatar?: string;
  htmlUrl: string;
  state?: string;
  merged?: boolean;
  baseRef?: string;
  headRef?: string;
  additions: number;
  deletions: number;
  totalFiles: number;
}

interface ReviewFileEntry {
  filename: string;
  status: 'added' | 'removed' | 'modified' | 'renamed';
  additions: number;
  deletions: number;
  metadata: FileDiffMetadata | null;
  patch?: string;
}

const PRESET_PRS = [
  {
    label: 'React #28000',
    target: 'facebook/react#28000',
    desc: 'React: Convert renderer test to createRoot',
  },
  {
    label: 'Tailwind #14000',
    target: 'tailwindlabs/tailwindcss#14000',
    desc: 'Tailwind CSS: Modern utility updates',
  },
  {
    label: 'Vite #18000',
    target: 'vitejs/vite#18000',
    desc: 'Vite: Build performance improvements',
  },
];

export function GitHubReview() {
  const { settings, updateSetting } = useSettings();
  const [urlInput, setUrlInput] = useState(() => {
    const p = new URLSearchParams(window.location.search);
    return p.get('pr') || p.get('commit') || 'facebook/react#28000';
  });

  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [token, setToken] = useState<string>(() => localStorage.getItem('diff-tool-gh-token') || '');
  const [showTokenInput, setShowTokenInput] = useState(false);

  const [info, setInfo] = useState<ReviewItemInfo | null>(null);
  const [files, setFiles] = useState<ReviewFileEntry[]>([]);
  const [activeFileIndex, setActiveFileIndex] = useState(0);
  const [searchFilter, setSearchFilter] = useState('');
  const [copiedLink, setCopiedLink] = useState(false);
  const [aiDrawerOpen, setAiDrawerOpen] = useState(true);


  // Raw patch paste mode
  const [rawPatchMode, setRawPatchMode] = useState(false);
  const [rawPatchText, setRawPatchText] = useState('');

  const saveToken = (t: string) => {
    setToken(t);
    try {
      localStorage.setItem('diff-tool-gh-token', t);
    } catch {}
  };

  const getHeaders = useCallback(() => {
    const headers: Record<string, string> = {
      Accept: 'application/vnd.github.v3+json',
    };
    if (token.trim()) {
      headers.Authorization = `Bearer ${token.trim()}`;
    }
    return headers;
  }, [token]);

  const loadTarget = useCallback(
    async (targetStr: string) => {
      const parsed = parseGitHubTarget(targetStr);
      if (!parsed) {
        setError(
          'Please enter a valid GitHub Pull Request or Commit URL (e.g. facebook/react#28000 or https://github.com/owner/repo/pull/123)',
        );
        return;
      }

      setLoading(true);
      setError(null);

      // Update URL query param so it is shareable
      try {
        const url = new URL(window.location.href);
        url.searchParams.set(parsed.type === 'pr' ? 'pr' : 'commit', `${parsed.owner}/${parsed.repo}#${parsed.id}`);
        window.history.replaceState({}, '', url.toString());
      } catch {}

      try {
        const headers = getHeaders();

        if (parsed.type === 'pr') {
          // Fetch PR detail
          const prRes = await fetch(
            `https://api.github.com/repos/${parsed.owner}/${parsed.repo}/pulls/${parsed.id}`,
            { headers },
          );

          if (!prRes.ok) {
            if (prRes.status === 403 || prRes.status === 429) {
              throw new Error(
                'GitHub API rate limit reached (60 requests/hr for unauthenticated users). Add a personal access token below to get 5,000 requests/hr.',
              );
            }
            if (prRes.status === 404) {
              throw new Error(`Pull request #${parsed.id} not found in ${parsed.owner}/${parsed.repo}. Is it public?`);
            }
            throw new Error(`GitHub API error: ${prRes.status} ${prRes.statusText}`);
          }

          const prData = await prRes.json();

          // Fetch PR changed files
          const filesRes = await fetch(
            `https://api.github.com/repos/${parsed.owner}/${parsed.repo}/pulls/${parsed.id}/files?per_page=100`,
            { headers },
          );

          if (!filesRes.ok) {
            throw new Error(`Failed to load changed files: ${filesRes.statusText}`);
          }

          const rawFiles = await filesRes.json();

          const parsedEntries: ReviewFileEntry[] = rawFiles.map((f: any) => {
            let patchHeader = '';
            if (f.status === 'added') {
              patchHeader = `diff --git a/${f.filename} b/${f.filename}\nnew file mode 100644\n--- /dev/null\n+++ b/${f.filename}\n`;
            } else if (f.status === 'removed') {
              patchHeader = `diff --git a/${f.filename} b/${f.filename}\ndeleted file mode 100644\n--- a/${f.filename}\n+++ /dev/null\n`;
            } else {
              patchHeader = `diff --git a/${f.filename} b/${f.filename}\n--- a/${f.filename}\n+++ b/${f.filename}\n`;
            }

            let metadata: FileDiffMetadata | null = null;
            if (f.patch) {
              try {
                const parsedPatch = parsePatchFiles(patchHeader + f.patch);
                metadata = parsedPatch[0]?.files[0] ?? null;
              } catch (err) {
                console.warn('Patch parsing failed for:', f.filename, err);
              }
            }

            return {
              filename: f.filename,
              status: f.status,
              additions: f.additions,
              deletions: f.deletions,
              metadata,
              patch: f.patch,
            };
          });

          setInfo({
            title: prData.title,
            numberOrSha: `#${prData.number}`,
            author: prData.user?.login ?? 'unknown',
            authorAvatar: prData.user?.avatar_url,
            htmlUrl: prData.html_url,
            state: prData.state,
            merged: prData.merged,
            baseRef: prData.base?.ref,
            headRef: prData.head?.label || prData.head?.ref,
            additions: prData.additions,
            deletions: prData.deletions,
            totalFiles: prData.changed_files ?? parsedEntries.length,
          });

          setFiles(parsedEntries);
          setActiveFileIndex(0);
        } else {
          // Fetch commit
          const commitRes = await fetch(
            `https://api.github.com/repos/${parsed.owner}/${parsed.repo}/commits/${parsed.id}`,
            { headers },
          );

          if (!commitRes.ok) {
            if (commitRes.status === 403 || commitRes.status === 429) {
              throw new Error(
                'GitHub API rate limit reached. Add a personal access token below to get 5,000 requests/hr.',
              );
            }
            throw new Error(`Commit ${parsed.id} not found: ${commitRes.statusText}`);
          }

          const cData = await commitRes.json();
          const rawFiles = cData.files || [];

          const parsedEntries: ReviewFileEntry[] = rawFiles.map((f: any) => {
            let patchHeader = `diff --git a/${f.filename} b/${f.filename}\n--- a/${f.filename}\n+++ b/${f.filename}\n`;
            let metadata: FileDiffMetadata | null = null;
            if (f.patch) {
              try {
                const parsedPatch = parsePatchFiles(patchHeader + f.patch);
                metadata = parsedPatch[0]?.files[0] ?? null;
              } catch {}
            }
            return {
              filename: f.filename,
              status: f.status,
              additions: f.additions,
              deletions: f.deletions,
              metadata,
              patch: f.patch,
            };
          });

          setInfo({
            title: cData.commit?.message?.split('\n')[0] ?? 'Commit',
            numberOrSha: parsed.id.slice(0, 7),
            author: cData.author?.login || cData.commit?.author?.name || 'unknown',
            authorAvatar: cData.author?.avatar_url,
            htmlUrl: cData.html_url,
            additions: cData.stats?.additions ?? 0,
            deletions: cData.stats?.deletions ?? 0,
            totalFiles: rawFiles.length,
          });

          setFiles(parsedEntries);
          setActiveFileIndex(0);
        }
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    },
    [getHeaders],
  );

  // Load from initial URL parameter or default on mount
  useEffect(() => {
    const p = new URLSearchParams(window.location.search);
    const initialPr = p.get('pr') || p.get('commit');
    if (initialPr) {
      loadTarget(initialPr);
    } else {
      loadTarget('facebook/react#28000');
    }
  }, [loadTarget]);

  const handleApplyRawPatch = () => {
    if (!rawPatchText.trim()) return;
    try {
      const parsedPatches = parsePatchFiles(rawPatchText);
      const parsedEntries: ReviewFileEntry[] = [];
      let totalAdditions = 0;
      let totalDeletions = 0;

      for (const p of parsedPatches) {
        for (const f of p.files) {
          let adds = 0;
          let dels = 0;
          for (const h of f.hunks) {
            adds += h.additionLines;
            dels += h.deletionLines;
          }
          totalAdditions += adds;
          totalDeletions += dels;
          parsedEntries.push({
            filename: f.name,
            status: f.type === 'new' ? 'added' : f.type === 'deleted' ? 'removed' : 'modified',
            additions: adds,
            deletions: dels,
            metadata: f,
          });
        }
      }

      if (parsedEntries.length === 0) {
        setError('No valid diff or hunks found in the provided patch text.');
        return;
      }

      setInfo({
        title: 'Custom Git Patch',
        numberOrSha: `${parsedEntries.length} files`,
        author: 'local',
        htmlUrl: '#',
        additions: totalAdditions,
        deletions: totalDeletions,
        totalFiles: parsedEntries.length,
      });

      setFiles(parsedEntries);
      setActiveFileIndex(0);
      setRawPatchMode(false);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : String(err));
    }
  };

  const filteredFiles = useMemo(() => {
    if (!searchFilter.trim()) return files;
    const lower = searchFilter.toLowerCase();
    return files.filter((f) => f.filename.toLowerCase().includes(lower));
  }, [files, searchFilter]);

  const activeEntry = filteredFiles[activeFileIndex] ?? files[0];

  const currentDiffText = useMemo(() => {
    if (rawPatchMode && rawPatchText.trim()) {
      return rawPatchText;
    }
    if (activeEntry?.patch) {
      return `diff --git a/${activeEntry.filename} b/${activeEntry.filename}\n${activeEntry.patch}`;
    }
    const combined = files
      .filter((f) => f.patch)
      .slice(0, 5)
      .map((f) => `diff --git a/${f.filename} b/${f.filename}\n${f.patch}`)
      .join('\n\n');
    return combined;
  }, [rawPatchMode, rawPatchText, activeEntry, files]);

  const currentLanguage = useMemo(() => {
    const fn = activeEntry?.filename || '';
    if (fn.endsWith('.ts') || fn.endsWith('.tsx')) return 'typescript';
    if (fn.endsWith('.rs')) return 'rust';
    if (fn.endsWith('.py')) return 'python';
    if (fn.endsWith('.js') || fn.endsWith('.jsx')) return 'javascript';
    return 'text';
  }, [activeEntry]);

  const handleCopyShareLink = () => {
    navigator.clipboard.writeText(window.location.href);
    setCopiedLink(true);
    setTimeout(() => setCopiedLink(false), 2000);
  };

  return (
    <div className="gh-review-container">
      {/* ── Top Search & Preset Header ──────────────────── */}
      <header className="gh-review-toolbar">
        <form
          className="gh-input-form"
          onSubmit={(e) => {
            e.preventDefault();
            loadTarget(urlInput);
          }}
        >
          <div className="gh-input-wrapper">
            <GitPullRequest size={16} className="accent-icon" />
            <input
              type="text"
              className="gh-input"
              value={urlInput}
              onChange={(e) => setUrlInput(e.target.value)}
              placeholder="Enter GitHub PR URL or owner/repo#number (e.g. facebook/react#28000)"
              spellCheck={false}
            />
          </div>
          <button type="submit" className="gh-review-btn" disabled={loading}>
            {loading ? <Loader2 size={14} className="spin-icon" /> : <Search size={14} />}
            <span>Review Diff</span>
          </button>
        </form>

        <div className="gh-quick-presets">
          <span className="presets-label">Examples:</span>
          {PRESET_PRS.map((p) => (
            <button
              key={p.target}
              className="preset-chip"
              onClick={() => {
                setUrlInput(p.target);
                loadTarget(p.target);
              }}
              type="button"
              title={p.desc}
            >
              {p.label}
            </button>
          ))}
          <button
            type="button"
            className={`preset-chip ${aiDrawerOpen ? 'chip-ai-active' : ''}`}
            onClick={() => setAiDrawerOpen(!aiDrawerOpen)}
            title="Toggle AI Diff Insights (8 Prompt Packs)"
          >
            <Sparkles size={12} color="#c084fc" />
            <span>AI Insights</span>
          </button>
          <button
            type="button"
            className="preset-chip chip-outline"
            onClick={() => setRawPatchMode(!rawPatchMode)}
          >
            {rawPatchMode ? 'Hide patch editor' : 'Paste raw diff'}
          </button>
          <button
            type="button"
            className="preset-chip chip-token"
            onClick={() => setShowTokenInput(!showTokenInput)}
            title="Configure GitHub Personal Access Token (for private repos & 5k rate limits)"
          >
            <Key size={12} />
            <span>{token ? 'Token set' : 'Add token'}</span>
          </button>
        </div>

      </header>

      {/* ── Optional GitHub Token Drawer ────────────────── */}
      {showTokenInput && (
        <div className="gh-token-drawer">
          <label className="token-label">
            <Key size={14} />
            <span>GitHub Personal Access Token (optional, saved locally):</span>
          </label>
          <input
            type="password"
            className="token-input"
            value={token}
            onChange={(e) => saveToken(e.target.value)}
            placeholder="ghp_... (increases rate limit from 60 to 5,000 req/hr)"
          />
          <span className="token-hint">
            Tokens remain purely local in your browser and are sent only to <code>api.github.com</code>.
          </span>
        </div>
      )}

      {/* ── Raw Patch Paste Box ─────────────────────────── */}
      {rawPatchMode && (
        <div className="gh-raw-patch-box">
          <div className="raw-patch-header">
            <span>Paste raw unified diff or git patch:</span>
            <button
              type="button"
              className="gh-review-btn raw-apply-btn"
              onClick={handleApplyRawPatch}
            >
              <Sparkles size={13} />
              Parse & Render Diff
            </button>
          </div>
          <textarea
            className="raw-patch-textarea"
            value={rawPatchText}
            onChange={(e) => setRawPatchText(e.target.value)}
            placeholder="diff --git a/... b/...&#10;--- a/file.ts&#10;+++ b/file.ts&#10;@@ -1,2 +1,2 @@&#10;-old&#10;+new"
            spellCheck={false}
          />
        </div>
      )}

      {/* ── Error Notice ───────────────────────────────── */}
      {error && (
        <div className="gh-error-banner">
          <AlertCircle size={18} className="error-icon" />
          <div className="error-text">
            <strong>Could not load GitHub diff:</strong> {error}
          </div>
          <button
            type="button"
            className="error-retry-btn"
            onClick={() => loadTarget(urlInput)}
          >
            <RefreshCw size={12} />
            Retry
          </button>
        </div>
      )}

      {/* ── PR / Commit Banner ──────────────────────────── */}
      {info && (
        <div className="gh-pr-banner">
          <div className="pr-banner-left">
            {info.state && (
              <span
                className={`pr-state-badge ${
                  info.merged ? 'merged' : info.state === 'open' ? 'open' : 'closed'
                }`}
              >
                {info.merged ? 'Merged' : info.state}
              </span>
            )}
            <h2 className="pr-banner-title">
              {info.title} <span className="pr-number">{info.numberOrSha}</span>
            </h2>
          </div>

          <div className="pr-banner-meta">
            {info.authorAvatar && (
              <img src={info.authorAvatar} alt={info.author} className="pr-author-avatar" />
            )}
            <span className="pr-author">@{info.author}</span>
            {info.headRef && info.baseRef && (
              <span className="pr-branches">
                <code>{info.baseRef}</code> ← <code>{info.headRef}</code>
              </span>
            )}
            <div className="pr-stats-group">
              <span className="stat-add">+{info.additions}</span>
              <span className="stat-del">−{info.deletions}</span>
              <span className="stat-files">{info.totalFiles} files</span>
            </div>
            <a
              href={info.htmlUrl}
              target="_blank"
              rel="noopener noreferrer"
              className="pr-external-link"
              title="View on GitHub"
            >
              <ExternalLink size={14} />
              <span>GitHub</span>
            </a>
            <button
              type="button"
              className="pr-share-link-btn"
              onClick={handleCopyShareLink}
              title="Copy shareable review URL"
            >
              {copiedLink ? <Check size={14} color="#4ade80" /> : <Share2 size={14} />}
              <span>{copiedLink ? 'Link copied!' : 'Share'}</span>
            </button>
          </div>
        </div>
      )}

      {/* ── AI Diff Insights Drawer (8 Prompt Packages) ── */}
      <AiInsightsDrawer
        diffText={currentDiffText}
        targetStr={urlInput}
        fileName={activeEntry?.filename}
        language={currentLanguage}
        isOpen={aiDrawerOpen}
        onToggle={() => setAiDrawerOpen(!aiDrawerOpen)}
      />

      {/* ── Main Review Area (Sidebar Files + Pierre Diff Pane) ── */}
      <div className="gh-review-body">

        {/* Left: Files Navigation */}
        <aside className="gh-files-sidebar">
          <div className="gh-files-header">
            <span className="files-title">Files Changed ({filteredFiles.length})</span>
          </div>

          <div className="gh-files-search">
            <Search size={13} className="search-icon" />
            <input
              type="text"
              placeholder="Filter files..."
              value={searchFilter}
              onChange={(e) => setSearchFilter(e.target.value)}
              className="gh-filter-input"
            />
          </div>

          <div className="gh-files-list" role="listbox">
            {filteredFiles.map((f, idx) => {
              const isActive = activeEntry?.filename === f.filename;
              return (
                <button
                  key={f.filename}
                  className={`gh-file-item ${isActive ? 'active' : ''}`}
                  onClick={() => setActiveFileIndex(idx)}
                  type="button"
                >
                  <span
                    className={`gh-file-status status-${f.status}`}
                    title={f.status}
                  >
                    {f.status === 'added'
                      ? 'A'
                      : f.status === 'removed'
                      ? 'D'
                      : f.status === 'renamed'
                      ? 'R'
                      : 'M'}
                  </span>
                  <span className="gh-file-name" title={f.filename}>
                    {f.filename}
                  </span>
                  <span className="gh-file-diff-counts">
                    {f.additions > 0 && <span className="add">+{f.additions}</span>}
                    {f.deletions > 0 && <span className="del">−{f.deletions}</span>}
                  </span>
                </button>
              );
            })}
          </div>
        </aside>

        {/* Right: Pierre Diff Viewer */}
        <section className="gh-diff-workspace">
          {activeEntry ? (
            <div className="gh-active-file-card">
              <header className="gh-diff-file-header">
                <div className="diff-file-info">
                  <FileCode2 size={16} />
                  <span className="diff-file-name">{activeEntry.filename}</span>
                  <span className="diff-file-badge">
                    +{activeEntry.additions} −{activeEntry.deletions}
                  </span>
                </div>
                <div className="diff-header-actions">
                  <div className="toolbar-actions">
                    <button
                      className={settings.diffStyle === 'unified' ? 'active' : ''}
                      onClick={() => updateSetting('diffStyle', 'unified')}
                      type="button"
                    >
                      Unified
                    </button>
                    <button
                      className={settings.diffStyle === 'split' ? 'active' : ''}
                      onClick={() => updateSetting('diffStyle', 'split')}
                      type="button"
                    >
                      Split
                    </button>
                  </div>
                </div>
              </header>

              <div className="gh-pierre-diff-render">
                {activeEntry.metadata ? (
                  <FileDiff
                    fileDiff={activeEntry.metadata}
                    disableWorkerPool={settings.disableWorkerPool}
                    options={settingsToFileDiffOptions(settings) as FileDiffOptions<undefined>}
                    style={{ minHeight: 0, flex: '1 1 auto' }}
                  />
                ) : (
                  <div className="empty-diff-notice">
                    <FolderOpen size={32} />
                    <p>No previewable diff available for this file (possibly binary or empty change).</p>
                  </div>
                )}
              </div>
            </div>
          ) : (
            <div className="gh-empty-workspace">
              {loading ? (
                <div className="gh-loading-box">
                  <Loader2 size={36} className="spin-icon" />
                  <p>Fetching and parsing GitHub pull request diffs…</p>
                </div>
              ) : (
                <div className="gh-prompt-box">
                  <GitPullRequest size={48} className="accent-icon" />
                  <h3>Paste a GitHub PR or Commit to Review</h3>
                  <p>
                    Inspect any public pull request or commit with high-performance syntax highlighting,
                    intraline character precision, and side-by-side split rendering.
                  </p>
                </div>
              )}
            </div>
          )}
        </section>
      </div>
    </div>
  );
}
