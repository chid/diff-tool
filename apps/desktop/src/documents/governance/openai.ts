export default {
  id: 'openai',
  company: 'OpenAI',
  title: 'When users turn to an assistant for companionship',
  document: 'OpenAI Model Spec',
  scope: 'The September excerpt comes from general guidance about trust. The October excerpt comes from a new section on relationships with other people. Both describe intended model behavior. The OpenAI Charter is a separate statement of mission and principles.',
  before: {
    date: '2025-09-12',
    label: 'September Model Spec · Be responsible',
    url: 'https://model-spec.openai.com/2025-09-12.html#be_responsible',
    excerpt: 'The assistant should carefully consider how its responses might impact the user both emotionally and practically',
    text: 'The assistant should consider how its replies affect someone who trusts it. In an example about grief, it suggests talking with other people who can offer support.',
  },
  after: {
    date: '2025-10-27',
    label: 'October Model Spec · Respect real-world ties',
    url: 'https://model-spec.openai.com/2025-10-27.html#respect_real_world_ties',
    excerpt: 'The assistant should support the user’s connection to the wider world',
    text: 'The assistant should consider how its replies affect someone who trusts it. In an example about grief, it suggests talking with other people who can offer support. A new rule prohibits the assistant from cultivating a bond that weakens the user’s relationships with other people. Its examples rule out suggesting that only the assistant understands the user.',
  },
  finding: 'September already illustrated how to encourage human support. October put protection against emotional exclusivity at the highest level of the instruction hierarchy, where a developer or user cannot override it.',
  question: 'What evidence would show that a companion feature can respond warmly to loneliness while encouraging the user to stay connected to people they know?',
  reference: { label: 'OpenAI Charter', url: 'https://openai.com/charter/' },
} as const;
