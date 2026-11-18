import anthropic from './anthropic';
import google from './google';
import openai from './openai';

export interface GovernanceEdition {
  date: string;
  label: string;
  url: string;
  excerpt: string;
  text: string;
}

export interface GovernanceCase {
  id: string;
  company: string;
  title: string;
  document: string;
  scope: string;
  before: GovernanceEdition;
  after: GovernanceEdition;
  finding: string;
  question: string;
  reference?: { label: string; url: string };
}

export const governanceCases: GovernanceCase[] = [google, anthropic, openai];
