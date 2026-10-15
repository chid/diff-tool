import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import {
  cleanJsonText,
  executePromptPackage,
  PROMPT_PACKAGES,
  PRECOMPUTED_INSIGHTS,
} from '../aiPromptPacks';

describe('cleanJsonText', () => {
  it('strips ```json markdown fences', () => {
    const raw = '```json\n{"summary": "Test summary"}\n```';
    expect(cleanJsonText(raw)).toBe('{"summary": "Test summary"}');
  });

  it('strips ``` markdown fences without language tag', () => {
    const raw = '```\n{"key": "value"}\n```';
    expect(cleanJsonText(raw)).toBe('{"key": "value"}');
  });

  it('strips <think>...</think> reasoning blocks', () => {
    const raw = '<think>I should analyze the diff carefully.\nLooks like a bug fix.</think>{"summary": "Fixes bug"}';
    expect(cleanJsonText(raw)).toBe('{"summary": "Fixes bug"}');
  });

  it('extracts innermost JSON object when chat model adds pre/postamble', () => {
    const raw = 'Here is the analysis:\n\n{"summary": "Update readme", "risks": []}\n\nHope this helps!';
    expect(cleanJsonText(raw)).toBe('{"summary": "Update readme", "risks": []}');
  });

  it('extracts JSON arrays when requested', () => {
    const raw = 'Sure! Here are questions:\n[\n  {"question": "Why?", "context": "None"}\n]';
    expect(cleanJsonText(raw)).toBe('[\n  {"question": "Why?", "context": "None"}\n]');
  });
});

describe('executePromptPackage with presets', () => {
  it('returns precomputed preset instantly when target matches and no apiKey is provided', async () => {
    const result = await executePromptPackage({
      packageId: 'full-review',
      diffText: 'diff --git a/test.js b/test.js',
      targetStr: 'facebook/react#28000',
    });

    expect(result.isPreset).toBe(true);
    expect(result.data).toEqual(PRECOMPUTED_INSIGHTS['facebook/react#28000']['full-review']);
  });

  it('falls back to default preset when no apiKey is provided and target is unknown', async () => {
    const result = await executePromptPackage({
      packageId: 'change-summary',
      diffText: 'diff --git a/unknown.js b/unknown.js',
    });

    expect(result.isPreset).toBe(true);
    expect(result.data).toEqual(PRECOMPUTED_INSIGHTS['facebook/react#28000']['change-summary']);
  });
});

describe('executePromptPackage with OpenRouter', () => {
  let fetchSpy: any;

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    fetchSpy.mockRestore();
  });

  it('sends correct headers, default model, and payload to OpenRouter endpoint', async () => {
    const mockResponsePayload = {
      choices: [
        {
          message: {
            content: '{"summary": "OpenRouter generated summary", "intents": ["refactor"], "risks": []}',
          },
        },
      ],
    };

    fetchSpy.mockResolvedValueOnce(
      new Response(JSON.stringify(mockResponsePayload), {
        status: 200,
        headers: { 'Content-Type': 'application/json' },
      })
    );

    const result = await executePromptPackage({
      packageId: 'change-summary',
      diffText: 'diff --git a/a.ts b/a.ts\n+const x = 1;',
      provider: 'openrouter',
      apiKey: 'sk-or-v1-abcdef123456',
    });

    expect(result.isPreset).toBe(false);
    expect(result.data.summary).toBe('OpenRouter generated summary');

    expect(fetchSpy).toHaveBeenCalledTimes(1);
    const [url, init] = fetchSpy.mock.calls[0];
    expect(url).toBe('https://openrouter.ai/api/v1/chat/completions');
    expect(init.method).toBe('POST');
    expect(init.headers['Authorization']).toBe('Bearer sk-or-v1-abcdef123456');
    expect(init.headers['HTTP-Referer']).toBe('https://github.com/charley/diff-tool');
    expect(init.headers['X-Title']).toBe('Diff Tool AI');

    const parsedBody = JSON.parse(init.body);
    expect(parsedBody.model).toBe('anthropic/claude-3.5-sonnet');
    expect(parsedBody.messages.length).toBe(2);
    expect(parsedBody.messages[0].role).toBe('system');
  });

  it('respects custom model and custom baseUrl for OpenRouter', async () => {
    const mockResponsePayload = {
      choices: [
        {
          message: {
            content: '```json\n{"risk_level": "Low", "justification": "Safe change", "affected_areas": []}\n```',
          },
        },
      ],
    };

    fetchSpy.mockResolvedValueOnce(
      new Response(JSON.stringify(mockResponsePayload), { status: 200 })
    );

    const result = await executePromptPackage({
      packageId: 'risk-assessment',
      diffText: 'diff --git a/docs/readme.md b/docs/readme.md',
      provider: 'openrouter',
      apiKey: 'sk-or-key',
      model: 'meta-llama/llama-3.3-70b-instruct',
      baseUrl: 'https://openrouter.proxy.local/v1/chat/completions',
    });

    expect(result.isPreset).toBe(false);
    expect(result.data.risk_level).toBe('Low');

    const [url, init] = fetchSpy.mock.calls[0];
    expect(url).toBe('https://openrouter.proxy.local/v1/chat/completions');
    const parsedBody = JSON.parse(init.body);
    expect(parsedBody.model).toBe('meta-llama/llama-3.3-70b-instruct');
  });

  it('throws an error on non-200 OpenRouter response', async () => {
    fetchSpy.mockResolvedValueOnce(
      new Response('Unauthorized: Invalid API key', {
        status: 401,
        statusText: 'Unauthorized',
      })
    );

    await expect(
      executePromptPackage({
        packageId: 'change-summary',
        diffText: 'diff --git a/file b/file',
        provider: 'openrouter',
        apiKey: 'bad-key',
      })
    ).rejects.toThrow(/OpenRouter API error: 401/);
  });
});

describe('executePromptPackage with localhost:8080 / custom provider', () => {
  let fetchSpy: any;

  beforeEach(() => {
    fetchSpy = vi.spyOn(globalThis, 'fetch');
  });

  afterEach(() => {
    fetchSpy.mockRestore();
  });

  it('defaults to http://localhost:8080/v1/chat/completions and handles unauthenticated local server', async () => {
    const llamaCppResponse = {
      choices: [
        {
          message: {
            content: '<think>Analyzing diff for commit message</think>{"subject": "fix(core): resolve null pointer", "body": "Checks null before access"}',
          },
        },
      ],
    };

    fetchSpy.mockResolvedValueOnce(
      new Response(JSON.stringify(llamaCppResponse), { status: 200 })
    );

    const result = await executePromptPackage({
      packageId: 'commit-message',
      diffText: 'diff --git a/src/main.rs b/src/main.rs\n+if let Some(x) = opt {',
      provider: 'custom',
      apiKey: '', // Empty local key
    });

    expect(result.isPreset).toBe(false);
    expect(result.data.subject).toBe('fix(core): resolve null pointer');

    const [url, init] = fetchSpy.mock.calls[0];
    expect(url).toBe('http://localhost:8080/v1/chat/completions');
    expect(init.headers['Authorization']).toBeUndefined();
    const parsedBody = JSON.parse(init.body);
    expect(parsedBody.model).toBe('default');
  });

  it('allows specifying custom localhost:8080 base url and model', async () => {
    const llamaCppResponse = {
      choices: [
        {
          message: {
            content: '{"summary": "Local llama 8080 summary", "intents": ["speedup"], "risks": []}',
          },
        },
      ],
    };

    fetchSpy.mockResolvedValueOnce(
      new Response(JSON.stringify(llamaCppResponse), { status: 200 })
    );

    const result = await executePromptPackage({
      packageId: 'change-summary',
      diffText: 'diff --git a/foo.rs b/foo.rs',
      provider: 'custom',
      apiKey: 'optional-local-token',
      baseUrl: 'http://127.0.0.1:8080/v1/chat/completions',
      model: 'qwen2.5-coder',
    });

    expect(result.isPreset).toBe(false);
    expect(result.data.summary).toBe('Local llama 8080 summary');

    const [url, init] = fetchSpy.mock.calls[0];
    expect(url).toBe('http://127.0.0.1:8080/v1/chat/completions');
    expect(init.headers['Authorization']).toBe('Bearer optional-local-token');
    const parsedBody = JSON.parse(init.body);
    expect(parsedBody.model).toBe('qwen2.5-coder');
  });
});

const env = (globalThis as { process?: { env?: Record<string, string | undefined> } }).process?.env ?? {};
const runLiveLlama = Boolean(env.TEST_LIVE_LLM || env.LIVE_LLM_TEST);
const openrouterKey = env.OPENROUTER_API_KEY;

describe.skipIf(!runLiveLlama)('Live localhost:8080 llama.cpp integration (opt-in with TEST_LIVE_LLM=1)', () => {
  it('connects to live localhost:8080 and returns valid JSON diff analysis', async () => {
    let isLive = false;
    try {
      const ping = await fetch('http://127.0.0.1:8080/v1/models', {
        signal: AbortSignal.timeout(1000),
      });
      isLive = ping.ok;
    } catch {
      isLive = false;
    }

    if (!isLive) {
      console.log('Skipping live localhost:8080 test: server is offline');
      return;
    }

    const result = await executePromptPackage({
      packageId: 'change-summary',
      diffText: 'diff --git a/src/math.rs b/src/math.rs\n+pub fn add(a: i32, b: i32) -> i32 { a + b }',
      provider: 'custom',
      baseUrl: 'http://127.0.0.1:8080/v1/chat/completions',
      apiKey: '',
    });

    expect(result.isPreset).toBe(false);
    expect(result.data).toBeDefined();
    expect(typeof result.data.summary === 'string' || typeof result.data === 'object').toBe(true);
  }, 45000);
});

describe.skipIf(!openrouterKey)('Live OpenRouter API integration (opt-in with OPENROUTER_API_KEY)', () => {
  it('connects to OpenRouter and performs live review report', async () => {
    const result = await executePromptPackage({
      packageId: 'risk-assessment',
      diffText: 'diff --git a/src/auth.ts b/src/auth.ts\n+export function verifyToken() { return true; }',
      provider: 'openrouter',
      apiKey: openrouterKey!,
    });

    expect(result.isPreset).toBe(false);
    expect(result.data).toBeDefined();
    expect(result.data.risk_level).toBeDefined();
  }, 30000);
});
