import { SplitSquareHorizontal, GitBranch, GitPullRequest, Layers3, TerminalSquare, ArrowRight } from 'lucide-react';

interface WelcomeHeroProps {
  /** Callback when user clicks explore examples button. */
  onExplore?: () => void;
  /** Callback when user clicks review GitHub PR button. */
  onReviewGitHub?: () => void;
}

const FEATURES = [
  {
    icon: GitPullRequest,
    title: 'GitHub PR & Commit Review',
    desc: 'Review any public GitHub PR or commit directly in the browser via Pierre diffs.',
  },
  {
    icon: GitBranch,
    title: 'Text diff & Intraline',
    desc: 'Myers & histogram algorithms with intraline character-level precision.',
  },
  {
    icon: Layers3,
    title: 'Folder scan',
    desc: 'Recursive folder comparison with tree navigation and diff badges.',
  },
  {
    icon: TerminalSquare,
    title: 'CLI + Desktop',
    desc: 'Deterministic Rust engine powers both the CLI and web/Tauri app.',
  },
];

export function WelcomeHero({ onExplore, onReviewGitHub }: WelcomeHeroProps) {
  return (
    <div className="welcome-hero">
      <section className="welcome-banner">
        <div className="welcome-brand">
          <SplitSquareHorizontal size={40} className="welcome-logo" />
          <h1 className="welcome-title">Diff Tool</h1>
          <span className="welcome-badge">v0.1.0</span>
        </div>
        <p className="welcome-tagline">
          Fast, accurate diffing — powered by Rust & Pierre diffs.
        </p>
        <p className="welcome-subtitle">
          Review GitHub pull requests, compare text files, and explore syntax-aware
          diffs with high-speed side-by-side split rendering.
        </p>
        <div className="welcome-actions">
          {onReviewGitHub && (
            <button className="welcome-btn welcome-btn-primary" onClick={onReviewGitHub} type="button">
              <GitPullRequest size={16} />
              Review GitHub PR
              <ArrowRight size={14} />
            </button>
          )}
          <button className="welcome-btn welcome-btn-secondary" onClick={onExplore} type="button">
            <Layers3 size={16} />
            Try Playground
          </button>
        </div>
      </section>

      <section className="welcome-features">
        {FEATURES.map((f) => {
          const Icon = f.icon;
          return (
            <div key={f.title} className="welcome-feature-card">
              <span className="welcome-feature-icon">
                <Icon size={22} />
              </span>
              <h3 className="welcome-feature-title">{f.title}</h3>
              <p className="welcome-feature-desc">{f.desc}</p>
            </div>
          );
        })}
      </section>

      <footer className="welcome-footer">
        <code className="welcome-cmd">
          $ dt diff left.txt right.txt --json
        </code>
        <span className="welcome-footer-hint">
          Also available as a CLI: <code>cargo run -p diff-cli</code>
        </span>
      </footer>
    </div>
  );
}
