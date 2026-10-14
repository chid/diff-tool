import { describe, it, expect, vi } from 'vitest';
import { render, screen } from '@testing-library/react';
import { parseGitHubTarget, GitHubReview } from '../components/GitHubReview';
import { SettingsProvider } from '../components/SettingsContext';

describe('parseGitHubTarget', () => {
  it('parses shorthand PR format owner/repo#123', () => {
    const res = parseGitHubTarget('facebook/react#28000');
    expect(res).toEqual({
      owner: 'facebook',
      repo: 'react',
      type: 'pr',
      id: '28000',
    });
  });

  it('parses shorthand commit format owner/repo@sha', () => {
    const res = parseGitHubTarget('chid/diff-tool@0afbd12');
    expect(res).toEqual({
      owner: 'chid',
      repo: 'diff-tool',
      type: 'commit',
      id: '0afbd12',
    });
  });

  it('parses full GitHub pull request URLs', () => {
    const res = parseGitHubTarget('https://github.com/tailwindlabs/tailwindcss/pull/14000');
    expect(res).toEqual({
      owner: 'tailwindlabs',
      repo: 'tailwindcss',
      type: 'pr',
      id: '14000',
    });
  });

  it('parses full GitHub commit URLs', () => {
    const res = parseGitHubTarget('https://github.com/torvalds/linux/commit/abcd1234ef56');
    expect(res).toEqual({
      owner: 'torvalds',
      repo: 'linux',
      type: 'commit',
      id: 'abcd1234ef56',
    });
  });

  it('returns null for empty or invalid input', () => {
    expect(parseGitHubTarget('')).toBeNull();
    expect(parseGitHubTarget('   ')).toBeNull();
    expect(parseGitHubTarget('random text without slash')).toBeNull();
  });
});

describe('GitHubReview component', () => {
  it('renders search input and preset buttons', () => {
    // Mock fetch to avoid real network call in tests
    vi.spyOn(globalThis, 'fetch').mockImplementation(() =>
      Promise.resolve(
        new Response(JSON.stringify({}), {
          status: 404,
          statusText: 'Not Found',
        }),
      ),
    );

    render(
      <SettingsProvider>
        <GitHubReview />
      </SettingsProvider>,
    );

    expect(screen.getByPlaceholderText(/Enter GitHub PR URL/i)).toBeInTheDocument();
    expect(screen.getByText('Review Diff')).toBeInTheDocument();
    expect(screen.getByText('React #28000')).toBeInTheDocument();
    expect(screen.getByText('Paste raw diff')).toBeInTheDocument();
  });
});
