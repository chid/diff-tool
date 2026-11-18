export default {
  id: 'google',
  company: 'Google',
  title: 'How can someone challenge an AI decision?',
  document: 'Google AI Principles',
  scope: 'A historical comparison of Google’s published AI design principles in June 2018 and February 2025. A product’s actual appeal process requires separate evidence.',
  before: {
    date: '2018-06-07',
    label: 'Original principles, objective 4',
    url: 'https://blog.google/innovation-and-ai/products/ai-principles/',
    excerpt: 'appropriate opportunities for feedback, relevant explanations, and appeal.',
    text: 'Google said its systems should give people appropriate ways to offer feedback, get an explanation and appeal. It also said people should retain suitable control over the technology.',
  },
  after: {
    date: '2025-02-04',
    label: 'Revised principles, page 3',
    url: 'https://ai.google/static/documents/EN_US-AI-Principles.pdf#page=3',
    excerpt: 'appropriate human oversight, due diligence, and feedback mechanisms',
    text: 'Google placed human oversight, due diligence and ways to give feedback within its approach to developing and deploying AI. It connected these practices to users’ goals, social responsibility and internationally accepted human rights principles.',
  },
  finding: 'The 2018 text names an appeal as one way a person could seek accountability. The 2025 text describes how teams should oversee systems and gather feedback across their working life.',
  question: 'For the product you plan to use, how can an affected person challenge a decision, who reviews the challenge, and what record shows the outcome?',
  reference: {
    label: 'Google’s announcement of the February 2025 revision',
    url: 'https://blog.google/innovation-and-ai/products/responsible-ai-2024-report-ongoing-work/',
  },
} as const;
