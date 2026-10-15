import { isTauri } from './tauriDetect';
import { compareAi } from './tauri';
import type { AiCompareRequest, AiProvider } from './types';

export interface PromptPackageMetadata {
  id: string;
  name: string;
  description: string;
  category: 'review' | 'audit' | 'authoring' | 'merge' | 'explain';
  version: number;
  default_temperature: number;
  default_max_tokens: number;
  tags: string[];
}

export interface AiChangeSummary {
  summary: string;
  intents: string[];
  risks: string[];
}

export interface AiRiskAssessment {
  risk_level: 'Low' | 'Medium' | 'High';
  justification: string;
  affected_areas: string[];
}

export interface AiReviewQuestion {
  question: string;
  context?: string;
}

export interface AiCommitMessage {
  subject: string;
  body?: string;
}

export interface AiFullReviewReport {
  summary: AiChangeSummary;
  risk: AiRiskAssessment;
  questions: AiReviewQuestion[];
  suggested_commit?: AiCommitMessage;
}

export interface AiHunkExplanation {
  explanation: string;
  key_points: string[];
}

export const PROMPT_PACKAGES: PromptPackageMetadata[] = [
  {
    id: 'full-review',
    name: 'Comprehensive Code Review',
    description: 'All-in-one report: executive summary, risk level, reviewer questions, and conventional commit suggestion.',
    category: 'review',
    version: 1,
    default_temperature: 0.25,
    default_max_tokens: 1536,
    tags: ['review', 'summary', 'risk', 'questions'],
  },
  {
    id: 'change-summary',
    name: 'Executive Change Summary',
    description: 'High-level summary of intent, bulleted changes, and notable risk signals.',
    category: 'review',
    version: 1,
    default_temperature: 0.3,
    default_max_tokens: 512,
    tags: ['summary', 'intent', 'tldr'],
  },
  {
    id: 'risk-assessment',
    name: 'Risk & Blast Radius Audit',
    description: 'Evaluates risk level (low/medium/high), justification, and affected modules or dependencies.',
    category: 'audit',
    version: 1,
    default_temperature: 0.2,
    default_max_tokens: 512,
    tags: ['risk', 'security', 'impact', 'audit'],
  },
  {
    id: 'review-questions',
    name: 'Targeted Reviewer Questions',
    description: 'Generates 3-5 critical questions focusing on edge cases, missing test fixtures, and security concerns.',
    category: 'review',
    version: 1,
    default_temperature: 0.4,
    default_max_tokens: 512,
    tags: ['questions', 'review', 'checklist'],
  },
  {
    id: 'commit-message',
    name: 'Conventional Commit Generator',
    description: 'Generates a structured conventional commit message (feat, fix, refactor) from staged or file diffs.',
    category: 'authoring',
    version: 1,
    default_temperature: 0.3,
    default_max_tokens: 512,
    tags: ['commit', 'git', 'conventional'],
  },
  {
    id: 'explain-hunk',
    name: 'Hunk / Line Change Explainer',
    description: 'Explains a specific diff hunk or selected code change in plain English.',
    category: 'explain',
    version: 1,
    default_temperature: 0.3,
    default_max_tokens: 256,
    tags: ['explain', 'hunk', 'education'],
  },
];

export const PRECOMPUTED_INSIGHTS: Record<
  string,
  {
    'full-review': AiFullReviewReport;
    'change-summary': AiChangeSummary;
    'risk-assessment': AiRiskAssessment;
    'review-questions': AiReviewQuestion[];
    'commit-message': AiCommitMessage;
    'explain-hunk': AiHunkExplanation;
  }
> = {
  'facebook/react#28000': {
    'full-review': {
      summary: {
        summary:
          'Migrates legacy ReactDOM.render calls to React 18 createRoot in test suites, wrapping mount and assertion passes in React.act() to eliminate synchronous lifecycle warnings.',
        intents: [
          'Modernize React test harness to use concurrent createRoot API',
          'Eliminate deprecation warnings in test logs',
          'Ensure consistent root teardown via root.unmount()',
        ],
        risks: [
          'Test suite timing differences due to asynchronous root rendering',
          'Ensuring act() boundaries enclose all microtask flushes',
        ],
      },
      risk: {
        risk_level: 'Low',
        justification:
          'Changes are strictly confined to test fixtures and assertions. Runtime package code and exported public APIs remain completely untouched.',
        affected_areas: ['packages/react-dom/src/__tests__', 'test-harness', 'fixtures/legacy-test'],
      },
      questions: [
        {
          question: 'Are there any lingering tests in the suite still relying on synchronous ReactDOM.render semantics?',
          context: 'Check package-level CI logs for deprecation notices.',
        },
        {
          question: 'Does root.unmount() run reliably inside afterEach even when assertions fail mid-test?',
          context: 'Prevents DOM leak across sequential test runs.',
        },
        {
          question: 'Have we verified compatibility if running against the React 19 testing pipeline?',
          context: 'Ensures future forward-compatibility of the test helpers.',
        },
      ],
      suggested_commit: {
        subject: 'test(react-dom): migrate renderer test harness to createRoot',
        body: 'Replaces deprecated ReactDOM.render calls with createRoot in renderer test files.\nWraps container operations in act() to ensure deterministic microtask flushing.',
      },
    },
    'change-summary': {
      summary:
        'Migrates legacy ReactDOM.render calls to React 18 createRoot in test suites, wrapping mount and assertion passes in React.act() to eliminate synchronous lifecycle warnings.',
      intents: [
        'Modernize React test harness to use concurrent createRoot API',
        'Eliminate deprecation warnings in test logs',
        'Ensure consistent root teardown via root.unmount()',
      ],
      risks: [
        'Test suite timing differences due to asynchronous root rendering',
      ],
    },
    'risk-assessment': {
      risk_level: 'Low',
      justification:
        'Changes are strictly confined to test fixtures and assertions. Runtime package code and exported public APIs remain completely untouched.',
      affected_areas: ['packages/react-dom/src/__tests__', 'test-harness', 'fixtures/legacy-test'],
    },
    'review-questions': [
      {
        question: 'Are there any lingering tests in the suite still relying on synchronous ReactDOM.render semantics?',
        context: 'Check package-level CI logs for deprecation notices.',
      },
      {
        question: 'Does root.unmount() run reliably inside afterEach even when assertions fail mid-test?',
        context: 'Prevents DOM leak across sequential test runs.',
      },
      {
        question: 'Have we verified compatibility if running against the React 19 testing pipeline?',
        context: 'Ensures future forward-compatibility of the test helpers.',
      },
    ],
    'commit-message': {
      subject: 'test(react-dom): migrate renderer test harness to createRoot',
      body: 'Replaces deprecated ReactDOM.render calls with createRoot in renderer test files.\nWraps container operations in act() to ensure deterministic microtask flushing.',
    },
    'explain-hunk': {
      explanation:
        'This change replaces the legacy ReactDOM.render(element, container) pattern with const root = createRoot(container); root.render(element). In React 18+, createRoot enables concurrent features and schedules rendering asynchronously.',
      key_points: [
        'Replaces legacy ReactDOM.render with createRoot',
        'Adds root.unmount() for deterministic cleanup',
        'Eliminates React 18 deprecation console warnings',
      ],
    },
  },
  'tailwindlabs/tailwindcss#14000': {
    'full-review': {
      summary: {
        summary:
          'Refactors dynamic class matching engine for subgrid and container query variants, improving compilation speed and reducing CSS bundle footprint.',
        intents: [
          'Add first-class support for native CSS @container query utilities',
          'Optimize AST compilation for arbitrary property syntax',
          'Update utility class index and type definitions',
        ],
        risks: [
          'Potential CSS rule ordering and specificity changes for pseudo-class variants',
          'Plugin compatibility with custom container query extensions',
        ],
      },
      risk: {
        risk_level: 'Medium',
        justification:
          'Modifies CSS generator pipeline core. While unit tests cover common utility syntax, subtle rule order differences could affect cascading specificity in complex layouts.',
        affected_areas: ['src/corePlugins.js', 'src/util/containerQueries.js', 'types/index.d.ts'],
      },
      questions: [
        {
          question: 'How does the arbitrary property parser handle nested bracket escaping (e.g. content-[\'[\'])?',
          context: 'Verify regex tokenizer correctly handles unbalanced inner braces.',
        },
        {
          question: 'Have we run regression visual diffs against existing Tailwind CSS v3 template projects?',
          context: 'Confirms generated stylesheet matches expected cascade order.',
        },
        {
          question: 'What is the benchmark impact on incremental build rebuild times with 10k+ utilities?',
          context: 'Ensures no polynomial time-complexity regressions in regex scanning.',
        },
      ],
      suggested_commit: {
        subject: 'feat(core): modernize container query utilities and optimize generator pipeline',
        body: 'Introduces enhanced container query variant resolution.\nRefactors arbitrary property tokenization for faster compilation.',
      },
    },
    'change-summary': {
      summary:
        'Refactors dynamic class matching engine for subgrid and container query variants, improving compilation speed and reducing CSS bundle footprint.',
      intents: [
        'Add first-class support for native CSS @container query utilities',
        'Optimize AST compilation for arbitrary property syntax',
        'Update utility class index and type definitions',
      ],
      risks: [
        'Potential CSS rule ordering and specificity changes for pseudo-class variants',
      ],
    },
    'risk-assessment': {
      risk_level: 'Medium',
      justification:
        'Modifies CSS generator pipeline core. While unit tests cover common utility syntax, subtle rule order differences could affect cascading specificity in complex layouts.',
      affected_areas: ['src/corePlugins.js', 'src/util/containerQueries.js', 'types/index.d.ts'],
    },
    'review-questions': [
      {
        question: 'How does the arbitrary property parser handle nested bracket escaping (e.g. content-[\'[\'])?',
        context: 'Verify regex tokenizer correctly handles unbalanced inner braces.',
      },
      {
        question: 'Have we run regression visual diffs against existing Tailwind CSS v3 template projects?',
        context: 'Confirms generated stylesheet matches expected cascade order.',
      },
      {
        question: 'What is the benchmark impact on incremental build rebuild times with 10k+ utilities?',
        context: 'Ensures no polynomial time-complexity regressions in regex scanning.',
      },
    ],
    'commit-message': {
      subject: 'feat(core): modernize container query utilities and optimize generator pipeline',
      body: 'Introduces enhanced container query variant resolution.\nRefactors arbitrary property tokenization for faster compilation.',
    },
    'explain-hunk': {
      explanation:
        'This modification updates the container variant parser to recognize named container identifiers (@[sidebar]/inline) in addition to default unnamed containers, generating appropriate @container CSS media rules.',
      key_points: [
        'Supports named container query identifiers (@[name])',
        'Generates standard CSS @container rule wrappers',
        'Preserves sorting order with existing responsive breakpoints',
      ],
    },
  },
  'vitejs/vite#18000': {
    'full-review': {
      summary: {
        summary:
          'Implements concurrent asset hashing during production Rollup bundle output and reduces redundant module graph graph-walks during hot module replacement (HMR).',
        intents: [
          'Accelerate production build times for multi-thousand asset projects',
          'Reduce HMR roundtrip latency by memoizing graph-walk ancestors',
          'Cache content-addressable hash buffers across compiler stages',
        ],
        risks: [
          'File descriptor exhaustion (EMFILE) on systems with low ulimits if worker pool is unbounded',
          'Hash cache invalidation race condition if file mtimes change rapidly',
        ],
      },
      risk: {
        risk_level: 'Medium',
        justification:
          'Touches production bundling pipeline and asset hash calculation. If asset hash calculation has platform discrepancies, CDN cache-busting could fail.',
        affected_areas: ['packages/vite/src/node/build.ts', 'packages/vite/src/node/server/moduleGraph.ts'],
      },
      questions: [
        {
          question: 'Is the concurrent asset hashing worker pool bounded to prevent EMFILE crashes on macOS?',
          context: 'Default max file descriptors on macOS is 256 in some shells.',
        },
        {
          question: 'Does the module graph walk correctly invalidate circular ESM dependencies during HMR?',
          context: 'Check circular module graph traversal depth guard.',
        },
        {
          question: 'Have we verified deterministic hash generation across Linux, macOS, and Windows line endings?',
          context: 'CRLF vs LF differences can alter chunk hashes unexpectedly.',
        },
      ],
      suggested_commit: {
        subject: 'perf(build): parallelize asset hashing and optimize HMR graph walk',
        body: 'Introduces worker-pool-based parallel hashing for output chunks.\nMemoizes upstream ancestor traversal during HMR client notifications.',
      },
    },
    'change-summary': {
      summary:
        'Implements concurrent asset hashing during production Rollup bundle output and reduces redundant module graph graph-walks during hot module replacement (HMR).',
      intents: [
        'Accelerate production build times for multi-thousand asset projects',
        'Reduce HMR roundtrip latency by memoizing graph-walk ancestors',
      ],
      risks: [
        'File descriptor exhaustion on systems with low ulimits',
      ],
    },
    'risk-assessment': {
      risk_level: 'Medium',
      justification:
        'Touches production bundling pipeline and asset hash calculation. If asset hash calculation has platform discrepancies, CDN cache-busting could fail.',
      affected_areas: ['packages/vite/src/node/build.ts', 'packages/vite/src/node/server/moduleGraph.ts'],
    },
    'review-questions': [
      {
        question: 'Is the concurrent asset hashing worker pool bounded to prevent EMFILE crashes on macOS?',
        context: 'Default max file descriptors on macOS is 256 in some shells.',
      },
      {
        question: 'Does the module graph walk correctly invalidate circular ESM dependencies during HMR?',
        context: 'Check circular module graph traversal depth guard.',
      },
      {
        question: 'Have we verified deterministic hash generation across Linux, macOS, and Windows line endings?',
        context: 'CRLF vs LF differences can alter chunk hashes unexpectedly.',
      },
    ],
    'commit-message': {
      subject: 'perf(build): parallelize asset hashing and optimize HMR graph walk',
      body: 'Introduces worker-pool-based parallel hashing for output chunks.\nMemoizes upstream ancestor traversal during HMR client notifications.',
    },
    'explain-hunk': {
      explanation:
        'This hunk switches the serial fs.readFileSync and crypto.createHash pipeline to use a bounded Promise pool with stream piping, computing sha256 checksums in parallel across multiple CPU cores.',
      key_points: [
        'Converts serial hash calculation to parallel chunk workers',
        'Uses stream-based hashing to minimize memory footprint',
        'Caches hash results keyed by mtime and size',
      ],
    },
  },
};

/**
 * Cleans markdown code fences, think tags, and conversational wrapper text from JSON output.
 */
export function cleanJsonText(raw: string): string {
  // Strip reasoning / thinking sections (e.g. DeepSeek-R1 / Qwen models)
  let text = raw.replace(/<think>[\s\S]*?<\/think>/gi, '').trim();
  // Strip markdown code fences
  text = text.replace(/^```(?:json)?\s*/i, '').replace(/```\s*$/, '').trim();

  // Find outermost valid JSON object or array bounds
  const firstBrace = text.indexOf('{');
  const firstBracket = text.indexOf('[');
  let start = -1;
  let end = -1;

  if (firstBrace !== -1 && (firstBracket === -1 || firstBrace < firstBracket)) {
    start = firstBrace;
    end = text.lastIndexOf('}');
  } else if (firstBracket !== -1) {
    start = firstBracket;
    end = text.lastIndexOf(']');
  }

  if (start !== -1 && end !== -1 && end > start) {
    text = text.slice(start, end + 1);
  }

  return text;
}

/**
 * Execute a prompt package on a diff, using either precomputed preset results or live LLM.
 */
export async function executePromptPackage(options: {
  packageId: string;
  diffText: string;
  targetStr?: string;
  language?: string;
  provider?: AiProvider;
  apiKey?: string;
  model?: string;
  baseUrl?: string;
}): Promise<{ data: any; isPreset: boolean }> {
  const { packageId, diffText, targetStr, language = 'text', provider = 'openai', apiKey, model, baseUrl } = options;

  // 1. If target is a known preset and no API key is specified (for cloud providers), return precomputed instantly
  if (targetStr && PRECOMPUTED_INSIGHTS[targetStr]) {
    const preset = PRECOMPUTED_INSIGHTS[targetStr];
    if (packageId in preset && provider !== 'custom' && (!apiKey || !apiKey.trim())) {
      return {
        data: (preset as any)[packageId],
        isPreset: true,
      };
    }
  }

  // 2. Fallback to generic precomputed React fixture if diff matches or no API key is present for cloud providers.
  // Custom local providers (e.g. localhost:8080 or Ollama) do not require an API key.
  if (provider !== 'custom' && (!apiKey || !apiKey.trim())) {
    const defaultPreset = PRECOMPUTED_INSIGHTS['facebook/react#28000'];
    if (packageId in defaultPreset) {
      return {
        data: (defaultPreset as any)[packageId],
        isPreset: true,
      };
    }
  }

  // 3. Live AI Execution (BYOK)
  const pkg = PROMPT_PACKAGES.find((p) => p.id === packageId);
  const systemPrompt =
    'You are an expert code reviewer and software architect. Analyze the provided git diff carefully and return ONLY a valid JSON object matching the requested schema without markdown code fences.';

  let userPrompt = '';
  if (packageId === 'full-review') {
    userPrompt = `Analyze this code change and generate a comprehensive review report.\n\nCode diff:\n${diffText.slice(0, 12000)}\n\nRespond with ONLY valid JSON:\n{\n  "summary": {\n    "summary": "1-2 sentence high-level overview",\n    "intents": ["list", "of", "intents"],\n    "risks": ["list", "of", "risks"]\n  },\n  "risk": {\n    "risk_level": "Low" | "Medium" | "High",\n    "justification": "why this rating was chosen",\n    "affected_areas": ["modules", "or", "files"]\n  },\n  "questions": [\n    {\n      "question": "Critical question for author",\n      "context": "Why this matters or edge case"\n    }\n  ],\n  "suggested_commit": {\n    "subject": "feat/fix(scope): brief description",\n    "body": "optional body explaining why"\n  }\n}`;
  } else if (packageId === 'change-summary') {
    userPrompt = `Summarize this diff in JSON:\n{\n  "summary": "high-level overview",\n  "intents": ["intent 1", "intent 2"],\n  "risks": ["risk 1"]\n}\n\nDiff:\n${diffText.slice(0, 12000)}`;
  } else if (packageId === 'risk-assessment') {
    userPrompt = `Assess risk for this diff in JSON:\n{\n  "risk_level": "Low" | "Medium" | "High",\n  "justification": "reasoning",\n  "affected_areas": ["files/modules"]\n}\n\nDiff:\n${diffText.slice(0, 12000)}`;
  } else if (packageId === 'review-questions') {
    userPrompt = `Generate 3-5 reviewer questions for this diff in JSON:\n[\n  {"question": "...", "context": "..."}\n]\n\nDiff:\n${diffText.slice(0, 12000)}`;
  } else if (packageId === 'commit-message') {
    userPrompt = `Generate a conventional commit message for this diff in JSON:\n{\n  "subject": "type(scope): subject",\n  "body": "optional explanation"\n}\n\nDiff:\n${diffText.slice(0, 12000)}`;
  } else if (packageId === 'explain-hunk') {
    userPrompt = `Explain this code change in plain English in JSON:\n{\n  "explanation": "Clear explanation paragraph",\n  "key_points": ["bullet 1", "bullet 2"]\n}\n\nDiff:\n${diffText.slice(0, 8000)}`;
  } else {
    userPrompt = `Analyze this code change:\n\n${diffText.slice(0, 10000)}`;
  }

  // A. If in Tauri desktop app, use Tauri IPC `compareAi`
  if (isTauri) {
    const defaultModel =
      provider === 'anthropic'
        ? 'claude-3-5-sonnet-latest'
        : provider === 'openrouter'
        ? 'anthropic/claude-3.5-sonnet'
        : provider === 'custom'
        ? 'default'
        : 'gpt-4o-mini';

    const req: AiCompareRequest = {
      provider,
      api_key: apiKey || '',
      model: model || defaultModel,
      base_url: baseUrl,
      temperature: pkg?.default_temperature ?? 0.3,
      max_tokens: pkg?.default_max_tokens ?? 1024,
      messages: [
        { role: 'system', content: systemPrompt },
        { role: 'user', content: userPrompt },
      ],
    };
    const res = await compareAi(req);
    const cleaned = cleanJsonText(res.content);
    return { data: JSON.parse(cleaned), isPreset: false };
  }

  // B. Browser web execution (fetch directly to OpenAI/Anthropic/OpenRouter/Local)
  if (provider === 'openai' || !provider) {
    const endpoint = baseUrl || 'https://api.openai.com/v1/chat/completions';
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
      },
      body: JSON.stringify({
        model: model || 'gpt-4o-mini',
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userPrompt },
        ],
        temperature: pkg?.default_temperature ?? 0.3,
        max_tokens: pkg?.default_max_tokens ?? 1024,
      }),
    });
    if (!response.ok) {
      throw new Error(`OpenAI API error: ${response.status} ${response.statusText}`);
    }
    const json = await response.json();
    const rawContent = json.choices?.[0]?.message?.content || '';
    return { data: JSON.parse(cleanJsonText(rawContent)), isPreset: false };
  } else if (provider === 'anthropic') {
    const endpoint = baseUrl || 'https://api.anthropic.com/v1/messages';
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        'x-api-key': apiKey || '',
        'anthropic-version': '2023-06-01',
        'dangerously-allow-browser': 'true',
      },
      body: JSON.stringify({
        model: model || 'claude-3-5-sonnet-latest',
        system: systemPrompt,
        messages: [{ role: 'user', content: userPrompt }],
        max_tokens: pkg?.default_max_tokens ?? 1024,
        temperature: pkg?.default_temperature ?? 0.3,
      }),
    });
    if (!response.ok) {
      throw new Error(`Anthropic API error: ${response.status} ${response.statusText}`);
    }
    const json = await response.json();
    const rawContent = json.content?.[0]?.text || '';
    return { data: JSON.parse(cleanJsonText(rawContent)), isPreset: false };
  } else if (provider === 'openrouter') {
    const endpoint = baseUrl || 'https://openrouter.ai/api/v1/chat/completions';
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        Authorization: `Bearer ${apiKey}`,
        'HTTP-Referer': 'https://github.com/charley/diff-tool',
        'X-Title': 'Diff Tool AI',
      },
      body: JSON.stringify({
        model: model || 'anthropic/claude-3.5-sonnet',
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userPrompt },
        ],
        temperature: pkg?.default_temperature ?? 0.3,
        max_tokens: pkg?.default_max_tokens ?? 1024,
      }),
    });
    if (!response.ok) {
      throw new Error(`OpenRouter API error: ${response.status} ${response.statusText}`);
    }
    const json = await response.json();
    const rawContent = json.choices?.[0]?.message?.content || '';
    return { data: JSON.parse(cleanJsonText(rawContent)), isPreset: false };
  } else {
    // Custom / Local endpoint (e.g. localhost:8080, Ollama)
    const endpoint = baseUrl || 'http://localhost:8080/v1/chat/completions';
    const response = await fetch(endpoint, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json',
        ...(apiKey ? { Authorization: `Bearer ${apiKey}` } : {}),
      },
      body: JSON.stringify({
        model: model || 'default',
        messages: [
          { role: 'system', content: systemPrompt },
          { role: 'user', content: userPrompt },
        ],
        temperature: pkg?.default_temperature ?? 0.3,
        max_tokens: pkg?.default_max_tokens ?? 1024,
      }),
    });
    if (!response.ok) {
      throw new Error(`Custom AI error: ${response.status} ${response.statusText}`);
    }
    const json = await response.json();
    const rawContent = json.choices?.[0]?.message?.content || '';
    return { data: JSON.parse(cleanJsonText(rawContent)), isPreset: false };
  }
}
