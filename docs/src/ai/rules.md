---
title: AI Rules in Zed - .rules, .cursorrules, CLAUDE.md
description: Configure AI behavior in Zed with .rules files, .cursorrules, CLAUDE.md, AGENTS.md, and other project-level instruction files.
---

# Using Rules {#using-rules}

Rules are prompts that can be inserted automatically at the beginning of each [Agent Panel](./agent-panel.md) interaction, through `.rules` files available in your project's file tree.

## `.rules` files

Zed supports including `.rules` files at the root of a project's file tree, and they act as project-level instructions that are auto-included in all of your interactions with the Agent Panel.

Other names for this file are also supported for compatibility with other agents, but note that the first file which matches in this list will be used:

- `.rules`
- `.cursorrules`
- `.windsurfrules`
- `.clinerules`
- `.github/copilot-instructions.md`
- `AGENT.md`
- `AGENTS.md`
- `CLAUDE.md`
- `GEMINI.md`

For guidance on writing effective rules:

- [Anthropic: Prompt Engineering](https://platform.claude.com/docs/en/build-with-claude/prompt-engineering/overview)
- [OpenAI: Prompt Engineering](https://platform.openai.com/docs/guides/prompt-engineering)
