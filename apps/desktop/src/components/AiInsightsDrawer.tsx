import { useState, useEffect, useCallback } from 'react';
import {
  Sparkles,
  ShieldAlert,
  HelpCircle,
  GitCommit,
  FileText,
  Lightbulb,
  Copy,
  Check,
  ChevronDown,
  ChevronUp,
  RefreshCw,
  Key,
  Sliders,
  AlertTriangle,
  CheckCircle2,
  XCircle,
} from 'lucide-react';
import {
  PROMPT_PACKAGES,
  executePromptPackage,
  type PromptPackageMetadata,
  type AiFullReviewReport,
  type AiChangeSummary,
  type AiRiskAssessment,
  type AiReviewQuestion,
  type AiCommitMessage,
  type AiHunkExplanation,
} from '../aiPromptPacks';

interface AiInsightsDrawerProps {
  diffText: string;
  targetStr?: string;
  fileName?: string;
  language?: string;
  isOpen: boolean;
  onToggle: () => void;
}

export function AiInsightsDrawer({
  diffText,
  targetStr,
  fileName,
  language = 'text',
  isOpen,
  onToggle,
}: AiInsightsDrawerProps) {
  const [activePackageId, setActivePackageId] = useState<string>('full-review');
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [result, setResult] = useState<any>(null);
  const [isPreset, setIsPreset] = useState(true);

  // Settings / BYOK
  const [showConfig, setShowConfig] = useState(false);
  const [provider, setProvider] = useState<'openai' | 'anthropic' | 'custom'>(
    () => (localStorage.getItem('diff-tool-ai-provider') as any) || 'openai'
  );
  const [apiKey, setApiKey] = useState<string>(
    () => localStorage.getItem('diff-tool-ai-key') || ''
  );
  const [model, setModel] = useState<string>(
    () => localStorage.getItem('diff-tool-ai-model') || ''
  );
  const [baseUrl, setBaseUrl] = useState<string>(
    () => localStorage.getItem('diff-tool-ai-base-url') || ''
  );

  // Copy feedbacks
  const [copiedSection, setCopiedSection] = useState<string | null>(null);

  const saveConfig = (p: 'openai' | 'anthropic' | 'custom', k: string, m: string, u: string) => {
    setProvider(p);
    setApiKey(k);
    setModel(m);
    setBaseUrl(u);
    try {
      localStorage.setItem('diff-tool-ai-provider', p);
      localStorage.setItem('diff-tool-ai-key', k);
      localStorage.setItem('diff-tool-ai-model', m);
      localStorage.setItem('diff-tool-ai-base-url', u);
    } catch {}
  };

  const runAnalysis = useCallback(
    async (packageId: string) => {
      setLoading(true);
      setError(null);
      try {
        const res = await executePromptPackage({
          packageId,
          diffText,
          targetStr,
          language,
          provider,
          apiKey,
          model: model.trim() || undefined,
          baseUrl: baseUrl.trim() || undefined,
        });
        setResult(res.data);
        setIsPreset(res.isPreset);
      } catch (err: any) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    },
    [diffText, targetStr, language, provider, apiKey, model, baseUrl]
  );

  // Auto-run when opened or package changes
  useEffect(() => {
    if (isOpen) {
      runAnalysis(activePackageId);
    }
  }, [isOpen, activePackageId, runAnalysis]);

  const handleCopy = (text: string, sectionKey: string) => {
    navigator.clipboard.writeText(text);
    setCopiedSection(sectionKey);
    setTimeout(() => setCopiedSection(null), 2000);
  };

  const getPackageIcon = (id: string) => {
    switch (id) {
      case 'full-review':
        return <Sparkles size={14} className="accent-sparkle" />;
      case 'change-summary':
        return <FileText size={14} />;
      case 'risk-assessment':
        return <ShieldAlert size={14} />;
      case 'review-questions':
        return <HelpCircle size={14} />;
      case 'commit-message':
        return <GitCommit size={14} />;
      case 'explain-hunk':
        return <Lightbulb size={14} />;
      default:
        return <Sparkles size={14} />;
    }
  };

  return (
    <div className={`ai-drawer-container ${isOpen ? 'open' : 'collapsed'}`}>
      {/* ── Collapsible Bar / Header ── */}
      <div className="ai-drawer-header">
        <button
          type="button"
          className="ai-drawer-toggle-btn"
          onClick={onToggle}
          title={isOpen ? 'Collapse AI Insights' : 'Expand AI Insights'}
        >
          <div className="ai-drawer-brand">
            <Sparkles size={16} className="sparkle-pulse" />
            <span className="ai-drawer-title">AI Diff Insights</span>
            <span className="ai-packages-badge">8 Prompt Packs</span>
          </div>
          {isOpen ? <ChevronUp size={16} /> : <ChevronDown size={16} />}
        </button>

        {isOpen && (
          <div className="ai-drawer-controls">
            <span className={`ai-origin-pill ${isPreset ? 'preset' : 'live'}`}>
              {isPreset ? '✨ Instant Demo Pack' : '⚡ Live Model'}
            </span>
            <button
              type="button"
              className="ai-ctrl-btn"
              onClick={() => setShowConfig(!showConfig)}
              title="Configure API key & model"
            >
              <Sliders size={13} />
              <span>{apiKey ? 'BYOK (Configured)' : 'Add API Key'}</span>
            </button>
            <button
              type="button"
              className="ai-ctrl-btn ai-run-btn"
              onClick={() => runAnalysis(activePackageId)}
              disabled={loading}
              title="Re-run analysis"
            >
              <RefreshCw size={13} className={loading ? 'spin-icon' : ''} />
              <span>{loading ? 'Analyzing…' : 'Run Pack'}</span>
            </button>
          </div>
        )}
      </div>

      {/* ── Expanded Content ── */}
      {isOpen && (
        <div className="ai-drawer-content">
          {/* Config Sub-panel */}
          {showConfig && (
            <div className="ai-config-panel">
              <div className="ai-config-row">
                <label className="ai-config-label">
                  <span>Provider:</span>
                  <select
                    value={provider}
                    onChange={(e) =>
                      saveConfig(e.target.value as any, apiKey, model, baseUrl)
                    }
                    className="ai-config-select"
                  >
                    <option value="openai">OpenAI (GPT-4o, GPT-4o-mini)</option>
                    <option value="anthropic">Anthropic (Claude 3.5 Sonnet)</option>
                    <option value="custom">Custom / Ollama (Local)</option>
                  </select>
                </label>

                <label className="ai-config-label flex-grow">
                  <span>API Key:</span>
                  <input
                    type="password"
                    placeholder="sk-... (stored locally in browser)"
                    value={apiKey}
                    onChange={(e) =>
                      saveConfig(provider, e.target.value, model, baseUrl)
                    }
                    className="ai-config-input"
                  />
                </label>

                <label className="ai-config-label">
                  <span>Model (optional):</span>
                  <input
                    type="text"
                    placeholder={
                      provider === 'anthropic'
                        ? 'claude-3-5-sonnet-latest'
                        : provider === 'custom'
                        ? 'llama3'
                        : 'gpt-4o-mini'
                    }
                    value={model}
                    onChange={(e) =>
                      saveConfig(provider, apiKey, e.target.value, baseUrl)
                    }
                    className="ai-config-input"
                  />
                </label>

                {provider === 'custom' && (
                  <label className="ai-config-label flex-grow">
                    <span>Base URL:</span>
                    <input
                      type="text"
                      placeholder="http://localhost:11434/v1/chat/completions"
                      value={baseUrl}
                      onChange={(e) =>
                        saveConfig(provider, apiKey, model, e.target.value)
                      }
                      className="ai-config-input"
                    />
                  </label>
                )}
              </div>
              <span className="ai-config-hint">
                Without an API key, instant high-quality precomputed insights are served for all demo PRs.
              </span>
            </div>
          )}

          {/* Package Selector Pills */}
          <div className="ai-packages-nav">
            {PROMPT_PACKAGES.map((pkg) => {
              const isActive = activePackageId === pkg.id;
              return (
                <button
                  key={pkg.id}
                  type="button"
                  className={`ai-pkg-chip ${isActive ? 'active' : ''}`}
                  onClick={() => {
                    setActivePackageId(pkg.id);
                  }}
                  title={pkg.description}
                >
                  {getPackageIcon(pkg.id)}
                  <span>{pkg.name}</span>
                </button>
              );
            })}
          </div>

          {/* Results Display */}
          <div className="ai-results-viewport">
            {loading ? (
              <div className="ai-loading-state">
                <RefreshCw size={24} className="spin-icon accent-icon" />
                <p>Generating {PROMPT_PACKAGES.find((p) => p.id === activePackageId)?.name}…</p>
              </div>
            ) : error ? (
              <div className="ai-error-state">
                <AlertTriangle size={18} className="error-icon" />
                <div className="ai-error-text">
                  <strong>Analysis failed:</strong> {error}
                </div>
              </div>
            ) : result ? (
              <div className="ai-report-body">
                {activePackageId === 'full-review' && (
                  <FullReviewView
                    report={result as AiFullReviewReport}
                    onCopy={handleCopy}
                    copiedSection={copiedSection}
                  />
                )}
                {activePackageId === 'change-summary' && (
                  <SummaryView summary={result as AiChangeSummary} />
                )}
                {activePackageId === 'risk-assessment' && (
                  <RiskView risk={result as AiRiskAssessment} />
                )}
                {activePackageId === 'review-questions' && (
                  <QuestionsView
                    questions={Array.isArray(result) ? result : (result.questions ?? [])}
                    onCopy={handleCopy}
                    copiedSection={copiedSection}
                  />
                )}
                {activePackageId === 'commit-message' && (
                  <CommitMessageView
                    commit={result as AiCommitMessage}
                    onCopy={handleCopy}
                    copiedSection={copiedSection}
                  />
                )}
                {activePackageId === 'explain-hunk' && (
                  <ExplainHunkView exp={result as AiHunkExplanation} />
                )}
              </div>
            ) : (
              <div className="ai-empty-prompt">
                <p>Select a prompt package above to analyze the diff.</p>
              </div>
            )}
          </div>
        </div>
      )}
    </div>
  );
}

/* ── Subviews for Specific Prompt Package Outputs ── */

function FullReviewView({
  report,
  onCopy,
  copiedSection,
}: {
  report: AiFullReviewReport;
  onCopy: (t: string, k: string) => void;
  copiedSection: string | null;
}) {
  const riskClass =
    report.risk?.risk_level === 'High'
      ? 'risk-high'
      : report.risk?.risk_level === 'Medium'
      ? 'risk-medium'
      : 'risk-low';

  const allQuestionsText = report.questions
    ?.map((q, i) => `${i + 1}. ${q.question}${q.context ? ` (${q.context})` : ''}`)
    .join('\n');

  const commitText = report.suggested_commit
    ? `${report.suggested_commit.subject}${
        report.suggested_commit.body ? `\n\n${report.suggested_commit.body}` : ''
      }`
    : '';

  return (
    <div className="ai-full-review-grid">
      {/* 1. Executive Summary */}
      <section className="ai-card summary-card">
        <div className="ai-card-header">
          <FileText size={15} className="card-icon" />
          <h4>Executive Summary</h4>
        </div>
        <p className="ai-summary-text">{report.summary?.summary}</p>

        {report.summary?.intents && report.summary.intents.length > 0 && (
          <div className="ai-subgroup">
            <span className="subgroup-title">Key Architectural Intents:</span>
            <ul className="ai-bullet-list">
              {report.summary.intents.map((item, i) => (
                <li key={i}>{item}</li>
              ))}
            </ul>
          </div>
        )}
      </section>

      {/* 2. Risk & Blast Radius */}
      <section className="ai-card risk-card">
        <div className="ai-card-header">
          <ShieldAlert size={15} className="card-icon" />
          <h4>Risk & Blast Radius</h4>
          <span className={`ai-risk-badge ${riskClass}`}>
            {report.risk?.risk_level || 'Low'} Risk
          </span>
        </div>
        <p className="ai-risk-justification">{report.risk?.justification}</p>

        {report.risk?.affected_areas && report.risk.affected_areas.length > 0 && (
          <div className="ai-subgroup">
            <span className="subgroup-title">Affected Modules & Fixtures:</span>
            <div className="ai-tag-group">
              {report.risk.affected_areas.map((area, i) => (
                <span key={i} className="ai-area-tag">
                  {area}
                </span>
              ))}
            </div>
          </div>
        )}
      </section>

      {/* 3. Reviewer Checklist Questions */}
      {report.questions && report.questions.length > 0 && (
        <section className="ai-card questions-card">
          <div className="ai-card-header">
            <HelpCircle size={15} className="card-icon" />
            <h4>Targeted Reviewer Questions</h4>
            {allQuestionsText && (
              <button
                type="button"
                className="ai-card-action-btn"
                onClick={() => onCopy(allQuestionsText, 'review-questions')}
                title="Copy all questions as checklist"
              >
                {copiedSection === 'review-questions' ? (
                  <Check size={12} color="#4ade80" />
                ) : (
                  <Copy size={12} />
                )}
                <span>
                  {copiedSection === 'review-questions' ? 'Copied!' : 'Copy Checklist'}
                </span>
              </button>
            )}
          </div>
          <ol className="ai-numbered-list">
            {report.questions.map((q, i) => (
              <li key={i} className="ai-question-item">
                <span className="q-text">{q.question}</span>
                {q.context && <span className="q-context">{q.context}</span>}
              </li>
            ))}
          </ol>
        </section>
      )}

      {/* 4. Suggested Conventional Commit */}
      {report.suggested_commit && (
        <section className="ai-card commit-card">
          <div className="ai-card-header">
            <GitCommit size={15} className="card-icon" />
            <h4>Suggested Conventional Commit</h4>
            <button
              type="button"
              className="ai-card-action-btn"
              onClick={() => onCopy(commitText, 'commit-msg')}
              title="Copy commit message"
            >
              {copiedSection === 'commit-msg' ? (
                <Check size={12} color="#4ade80" />
              ) : (
                <Copy size={12} />
              )}
              <span>{copiedSection === 'commit-msg' ? 'Copied!' : 'Copy'}</span>
            </button>
          </div>
          <div className="ai-commit-preview">
            <div className="commit-subject">{report.suggested_commit.subject}</div>
            {report.suggested_commit.body && (
              <div className="commit-body">{report.suggested_commit.body}</div>
            )}
          </div>
        </section>
      )}
    </div>
  );
}

function SummaryView({ summary }: { summary: AiChangeSummary }) {
  return (
    <div className="ai-single-card-view">
      <div className="ai-card">
        <h4 className="view-title">Executive Summary</h4>
        <p className="ai-summary-text">{summary.summary}</p>

        {summary.intents && summary.intents.length > 0 && (
          <div className="ai-subgroup">
            <span className="subgroup-title">Intents:</span>
            <ul className="ai-bullet-list">
              {summary.intents.map((item, i) => (
                <li key={i}>{item}</li>
              ))}
            </ul>
          </div>
        )}

        {summary.risks && summary.risks.length > 0 && (
          <div className="ai-subgroup">
            <span className="subgroup-title">Identified Risk Signals:</span>
            <ul className="ai-bullet-list risks">
              {summary.risks.map((item, i) => (
                <li key={i}>{item}</li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}

function RiskView({ risk }: { risk: AiRiskAssessment }) {
  const riskClass =
    risk.risk_level === 'High'
      ? 'risk-high'
      : risk.risk_level === 'Medium'
      ? 'risk-medium'
      : 'risk-low';

  return (
    <div className="ai-single-card-view">
      <div className="ai-card">
        <div className="ai-card-header">
          <h4 className="view-title">Risk Assessment & Blast Radius</h4>
          <span className={`ai-risk-badge ${riskClass}`}>{risk.risk_level} Risk</span>
        </div>
        <p className="ai-risk-justification">{risk.justification}</p>

        {risk.affected_areas && risk.affected_areas.length > 0 && (
          <div className="ai-subgroup">
            <span className="subgroup-title">Affected Modules & Dependencies:</span>
            <div className="ai-tag-group">
              {risk.affected_areas.map((area, i) => (
                <span key={i} className="ai-area-tag">
                  {area}
                </span>
              ))}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

function QuestionsView({
  questions,
  onCopy,
  copiedSection,
}: {
  questions: AiReviewQuestion[];
  onCopy: (t: string, k: string) => void;
  copiedSection: string | null;
}) {
  const allText = questions
    .map((q, i) => `${i + 1}. ${q.question}${q.context ? ` (${q.context})` : ''}`)
    .join('\n');

  return (
    <div className="ai-single-card-view">
      <div className="ai-card">
        <div className="ai-card-header">
          <h4 className="view-title">Reviewer Questions</h4>
          <button
            type="button"
            className="ai-card-action-btn"
            onClick={() => onCopy(allText, 'all-questions')}
          >
            {copiedSection === 'all-questions' ? (
              <Check size={12} color="#4ade80" />
            ) : (
              <Copy size={12} />
            )}
            <span>{copiedSection === 'all-questions' ? 'Copied!' : 'Copy All'}</span>
          </button>
        </div>
        <ol className="ai-numbered-list">
          {questions.map((q, i) => (
            <li key={i} className="ai-question-item">
              <span className="q-text">{q.question}</span>
              {q.context && <span className="q-context">{q.context}</span>}
            </li>
          ))}
        </ol>
      </div>
    </div>
  );
}

function CommitMessageView({
  commit,
  onCopy,
  copiedSection,
}: {
  commit: AiCommitMessage;
  onCopy: (t: string, k: string) => void;
  copiedSection: string | null;
}) {
  const text = `${commit.subject}${commit.body ? `\n\n${commit.body}` : ''}`;

  return (
    <div className="ai-single-card-view">
      <div className="ai-card">
        <div className="ai-card-header">
          <h4 className="view-title">Suggested Conventional Commit</h4>
          <button
            type="button"
            className="ai-card-action-btn"
            onClick={() => onCopy(text, 'commit-alone')}
          >
            {copiedSection === 'commit-alone' ? (
              <Check size={12} color="#4ade80" />
            ) : (
              <Copy size={12} />
            )}
            <span>{copiedSection === 'commit-alone' ? 'Copied!' : 'Copy'}</span>
          </button>
        </div>
        <div className="ai-commit-preview">
          <div className="commit-subject">{commit.subject}</div>
          {commit.body && <div className="commit-body">{commit.body}</div>}
        </div>
      </div>
    </div>
  );
}

function ExplainHunkView({ exp }: { exp: AiHunkExplanation }) {
  return (
    <div className="ai-single-card-view">
      <div className="ai-card">
        <h4 className="view-title">Code Change Explanation</h4>
        <p className="ai-summary-text">{exp.explanation}</p>
        {exp.key_points && exp.key_points.length > 0 && (
          <div className="ai-subgroup">
            <span className="subgroup-title">Key Takeaways:</span>
            <ul className="ai-bullet-list">
              {exp.key_points.map((pt, i) => (
                <li key={i}>{pt}</li>
              ))}
            </ul>
          </div>
        )}
      </div>
    </div>
  );
}
