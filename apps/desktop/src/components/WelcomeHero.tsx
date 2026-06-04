import { SplitSquareHorizontal, GitBranch, Layers3, Image, TerminalSquare, ArrowRight } from 'lucide-react';

interface WelcomeHeroProps {
  /** Callback when user clicks one of the CTA buttons. */
  onExplore?: () => void;
}

const FEATURES = [
  {
    icon: GitBranch,
    title: 'Text diff',
    desc: 'Myers & histogram algorithms with intraline character-level spans.',
  },
  {
    icon: Layers3,
    title: 'Folder scan',
    desc: 'Single-root recursive scans with tree navigation for the MVP.',
  },
  {
    icon: Image,
    title: 'Roadmap modes',
    desc: 'Image, merge, Git, Office, and AI workflows stay post-MVP.',
  },
  {
    icon: TerminalSquare,
    title: 'CLI + Desktop',
    desc: 'Same Rust engine powers both the CLI and native Tauri desktop app.',
  },
];

export function WelcomeHero({ onExplore }: WelcomeHeroProps) {
  return (
    <div className="welcome-hero">
      <section className="welcome-banner">
        <div className="welcome-brand">
          <SplitSquareHorizontal size={40} className="welcome-logo" />
          <h1 className="welcome-title">Diff Tool</h1>
          <span className="welcome-badge">v0.1.0</span>
        </div>
        <p className="welcome-tagline">
          Fast, accurate text diffing — powered by Rust, rendered with React.
        </p>
        <p className="welcome-subtitle">
          Compare text files and scan local folders with a native desktop
          experience. Open two files or drag-and-drop one folder to get started.
        </p>
        <div className="welcome-actions">
          <button className="welcome-btn welcome-btn-primary" onClick={onExplore} type="button">
            <Layers3 size={16} />
            Explore live examples
            <ArrowRight size={14} />
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
