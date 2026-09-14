# OpenAI Codex - Structured Summary

## Overview
OpenAI Codex is OpenAI's software engineering agent, relaunched in May 2025 as a cloud-based agent that can work on many coding tasks in parallel rather than just suggesting completions.

**Key Quote**: "Given a repository, it writes features, fixes bugs, answers questions about a codebase, runs tests, and proposes changes (including GitHub pull requests) for developer review."

## Architecture & Technical Details

### Core Architecture
- **Relaunch**: May 16, 2025 (research preview)
- **Original Model**: codex-1, described as a version of OpenAI's o3 reasoning model optimized for software engineering
- **Current Models**: Ships on later Codex-tuned models
- **Execution Environment**: Isolated, sandboxed cloud environments preloaded with user's code

### Technical Capabilities
- **Parallel Processing**: Works on many tasks at once inside isolated, sandboxed cloud environments
- **Repository Understanding**: Reads files, edits code, and runs commands and tests
- **Output Format**: Returns diffs and command/test logs for inspection
- **Integration**: Can open changes as GitHub pull requests for human review

## Capabilities

### Core Functions
1. **Run coding tasks in parallel in cloud sandboxes** (Supervised)
2. **Write features and fix bugs** (Supervised)
3. **Propose GitHub pull requests** (Supervised)
4. **Answer questions about a codebase and review code** (Copilot)
5. **Run in the terminal via the open-source CLI** (Supervised)

### CLI Features
- Runs locally in the terminal
- Configurable sandbox modes: read-only, workspace-write, full-access
- Approval policies for human control
- Supports MCP (Model Context Protocol) for tool integration

### Integration Surfaces
- **Codex Web**: chatgpt.com/codex
- **CLI**: Open-source command-line interface
- **IDE Extensions**: VS Code, JetBrains, and Xcode
- **Desktop App**: Windows and macOS
- **Platform Integrations**: GitHub, Slack, and Linear

## Pricing Structure

### Included in ChatGPT Plans
- **Free**: $0
- **Go**: $8/month
- **Plus**: $20/month
- **Pro**: From $100/month (with 5x and 20x rate-limit tiers)
- **Business**: Available
- **Edu**: Available
- **Enterprise**: Available

### Usage Model
- **Cloud Usage**: Metered per rolling 5-hour window, scales by plan tier
- **API Option**: CLI can run against an OpenAI API key billed at standard per-token API rates

## Limitations

### Autonomy Constraints
- **Human Review Required**: Agentic tasks require human review and approval; not a hands-off autonomous engineer
- **Supervised Agent**: Performs multi-step work autonomously inside sandbox but surfaces diffs, logs, and pull requests for developer approval

### Technical Limitations
- **Rate Limiting**: Cloud usage is rate-limited per 5-hour window and scales with plan tier
- **Vendor Claims**: Reliability and benchmark claims are vendor-reported
- **Model Lock-in**: Tied to OpenAI models (not model-agnostic)

### Use Case Limitations
- **Not for Unattended Automation**: Less suited to fully unattended automation where no human reviews the agent's output
- **Not for Model-Agnostic Needs**: Not suitable for teams that prefer a model-agnostic tool

## Strengths

1. **Unified Experience**: One coding agent across cloud, terminal, IDE, and GitHub from a single account
2. **Parallel Processing**: Cloud tasks run in parallel in isolated sandboxes and return reviewable diffs and test logs
3. **Open-Source CLI**: Configurable sandbox and approval modes, plus MCP support
4. **Bundled Pricing**: Included in paid ChatGPT plans rather than a separate purchase

## Best Use Cases

### Ideal For
- Developers and teams who want one OpenAI coding agent across terminal, IDE, cloud, and GitHub
- Parallelizable tasks: bug fixes, refactors, test passes
- Work reviewed via diffs and PRs

### Not Ideal For
- Fully unattended automation where no human reviews output
- Teams preferring model-agnostic tools
- Scenarios requiring hands-off autonomous engineering

## Alternatives & Competition

### Main Competitors
- **Claude Code** (Anthropic)
- **Cursor** (AI-powered code editor)
- **GitHub Copilot** (GitHub/Microsoft)
- **Cognition's Devin** (AI software engineer)
- **Amazon Q Developer** (AWS)
- **Aider** (AI pair programming)

### Differentiation
Codex differentiates on being OpenAI's first-party agent spanning cloud, CLI, IDE, and GitHub from a single ChatGPT account.

## Adoption & Metrics

### User Base
- **Reported**: 2 million weekly active users by early 2026 (vendor-reported)
- **Note**: Treat adoption and benchmark figures as vendor-reported

## Technical Specifications

### Sandbox Modes (CLI)
1. **Read-only**: Limited file access
2. **Workspace-write**: Write access to workspace
3. **Full-access**: Complete system access

### Configuration
- Local config file for sandbox mode, approval policy, and similar settings
- MCP support for tool integration

## Key Quotes from Source

1. "OpenAI Codex is OpenAI's software engineering agent, relaunched in May 2025 as a cloud-based agent that can work on many coding tasks in parallel rather than just suggesting completions."

2. "Given a repository, it writes features, fixes bugs, answers questions about a codebase, runs tests, and proposes changes (including GitHub pull requests) for developer review."

3. "Codex is a supervised agent. It performs multi-step work autonomously inside its sandbox but surfaces diffs, logs, and pull requests for a developer to approve before changes land."

4. "Best for developers and teams who want one OpenAI coding agent across the terminal, IDE, cloud, and GitHub, especially for parallelizable tasks (bug fixes, refactors, test passes) reviewed via diffs and PRs."

5. "Less suited to fully unattended automation where no human reviews the agent's output, or to teams that prefer a model-agnostic tool (Codex is tied to OpenAI models)."

## Sources & References
- **Primary Source**: AI Agents Wiki (https://aiagents.wiki/agents/openai-codex)
- **Last Reviewed**: 2026-06-20
- **Wikipedia Reference**: https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent)
- **Developer Docs**: https://developers.openai.com/codex

## Summary
OpenAI Codex represents OpenAI's entry into the AI coding agent space, offering a supervised, parallel-processing approach to software engineering tasks. While it provides powerful capabilities across multiple interfaces and is bundled with ChatGPT plans, it requires human oversight and is limited to OpenAI's model ecosystem. It's best suited for teams wanting integrated AI assistance across their development workflow rather than fully autonomous coding solutions.