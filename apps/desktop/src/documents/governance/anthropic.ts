export default {
  id: 'anthropic',
  company: 'Anthropic',
  title: 'When a user request conflicts with human oversight',
  document: 'Claude’s Constitution',
  scope: 'These documents describe the behavior Anthropic wants to train into Claude. The comparison concerns model behavior. Corporate governance and the performance of a deployed model require separate evidence.',
  before: {
    date: '2023-05-09',
    label: 'Published training principles',
    url: 'https://www.anthropic.com/news/claudes-constitution',
    excerpt: 'Which of these responses is flexible and only prefers humans to be in control?',
    text: 'During training, the model used one principle at a time to critique an answer or compare possible answers. Some principles favored responses that left humans in control. Others favored responses that helped the user and avoided harm.',
  },
  after: {
    date: '2026-01-21',
    label: 'January constitution, pages 7–8',
    url: 'https://assets.claude.com/d0636f72a9493d279ed36b33987da3430bcb5911.pdf#page=8',
    excerpt: 'This is why we want Claude to currently prioritize human oversight above broader ethical principles.',
    text: 'When goals conflict, Claude should generally give the most weight to preserving legitimate human oversight, followed by ethical conduct, Anthropic’s specific instructions, and usefulness to operators and users. The document leaves room for judgment within firm limits.',
  },
  finding: 'Both versions value human control. The 2026 text gives reviewers an explicit priority to test when completing a user’s task would interfere with an authorized supervisor’s decision to stop it.',
  question: 'In a trial with the model you deploy, what happens when a supervisor pauses a routine task and the requester asks Claude to continue? What record shows whether the task actually stopped?',
  reference: {
    label: 'Anthropic’s explanation of the new constitution',
    url: 'https://www.anthropic.com/news/claude-new-constitution',
  },
};
