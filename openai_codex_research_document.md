# OpenAI Codex: Comprehensive Research Document

## Table of Contents
1. [Current State of OpenAI Codex](#current-state-of-openai-codex)
2. [OpenAI Codex CLI - Terminal-Based Coding Agent](#openai-codex-cli---terminal-based-coding-agent)
3. [Codex Capabilities](#codex-capabilities)
4. [Architecture and Technical Foundation](#architecture-and-technical-foundation)
5. [Integration with ChatGPT and Other OpenAI Products](#integration-with-chatgpt-and-other-openai-products)
6. [Cloud-Based Code Execution Capabilities](#cloud-based-code-execution-capabilities)
7. [Comparison with GitHub Copilot](#comparison-with-github-copilot)
8. [OpenAI's Coding Strategy](#openais-coding-strategy)
9. [Technical Specifications and Limitations](#technical-specifications-and-limitations)
10. [Pricing and Availability](#pricing-and-availability)

---

## Current State of OpenAI Codex

### What Happened to the Original Codex Model

The original OpenAI Codex model, which was introduced in 2021 as an AI system that could translate natural language to code, has been largely superseded by newer models. According to the research, the original Codex model was trained on billions of lines of public code and could understand and generate code in dozens of programming languages. It was available via API for developers to integrate into their applications.

**Current Status**: The original Codex model has been replaced by more advanced models in the GPT family. As noted in the research, "OpenAI has since released GPT-4 and other models that have largely superseded Codex." The original Codex model served as the foundation for GitHub Copilot, but has evolved into a much more sophisticated system.

### Evolution Timeline

The Codex brand has been repurposed for a comprehensive AI coding agent platform:

- **April 16, 2025**: Initial release as Codex CLI (open-source, terminal-based)
- **May 16, 2025**: Codex Cloud research preview announced
- **February 2026**: Desktop app released for Windows and macOS
- **July 9, 2026**: Merged with ChatGPT desktop app into "superapp"

**Key Quote**: "By March 2026, Codex had grown to more than 2 million weekly active users." - [Wikipedia](https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent))

---

## OpenAI Codex CLI - Terminal-Based Coding Agent

### Overview

OpenAI Codex CLI is a lightweight, open-source coding agent that runs locally in your terminal. It allows developers to inspect, edit, and run code without leaving their terminal environment.

**Source**: [OpenAI Codex GitHub Repository](https://github.com/openai/codex)

### Key Features

1. **Local Repository Work**: Works directly with your local codebase
2. **Multi-Model Support**: Choose from multiple GPT models with different capability/speed tradeoffs
3. **Terminal Integration**: Full terminal workflow with commands
4. **Skills & Plugins**: Extensible with skills and plugins for team tools and data
5. **Code Review**: Built-in review capabilities for uncommitted changes, commits, or base branches
6. **Cloud Integration**: Can move work to Codex cloud and return results to local repository
7. **MCP Support**: Connect external tools via Model Context Protocol

### Installation & Setup

```bash
# macOS/Linux
curl -fsSL https://chatgpt.com/codex/install.sh | sh

# Windows
powershell -ExecutionPolicy ByPass -c "irm https://chatgpt.com/codex/install.ps1 | iex"

# Via npm
npm install -g @openai/codex

# Via Homebrew
brew install --cask codex
```

**Technical Details**:
- **Language**: Primarily Rust (96.6%), with Python (2.7%), TypeScript (0.2%)
- **License**: Apache-2.0
- **Repository Stats**: 124k stars, 617 contributors, 19.1k forks

### Core Commands

```bash
codex                    # Start interactive session
codex exec "prompt"      # Non-interactive execution
codex resume             # Continue saved session
codex fork               # Create alternative branch
codex review             # Code review mode
codex mcp add <server>   # Add MCP server
codex login              # Authentication
```

### Authentication Options

1. **ChatGPT Account**: OAuth-based login (recommended for Plus, Pro, Business, Edu, Enterprise)
2. **API Key**: Manual setup via `CODEX_API_KEY` environment variable

**Key Quote**: "Inspect code, make changes, run commands, and automate repeatable work without leaving your terminal." - [OpenAI Documentation](https://developers.openai.com/codex/cli)

---

## Codex Capabilities

### Core Functions

1. **Code Generation**: Generate features and implement new code from natural language
2. **Bug Fixing**: Identify and resolve software bugs autonomously
3. **Code Review**: Propose code changes for human review with detailed analysis
4. **Codebase Analysis**: Answer questions about existing code and navigate complex codebases
5. **Test Execution**: Run tests and validate code changes
6. **File Operations**: Read, edit, and manage project files
7. **Command Execution**: Run terminal commands and scripts
8. **Security Scanning**: Identify vulnerabilities through Codex Security

### Advanced Features

- **Parallel Task Execution**: Can work on multiple tasks simultaneously
- **Computer Use**: Can operate your computer by seeing, clicking, and typing
- **Memory & Learning**: Remembers preferences and learns from previous actions
- **Plugin Support**: 90+ plugins for tool integration (JIRA, GitLab, Slack, etc.)
- **Background Operation**: Capable of "always-on background work" for routine tasks

### Task Execution Model

Each task runs in a separate cloud environment with the following workflow:

1. **Container Creation**: Creates isolated containers for each cloud chat session
2. **Repository Checkout**: Automatically checks out your repository at selected branch/commit
3. **Setup Scripts**: Runs custom setup scripts for dependencies and tools
4. **Agent Execution**: Runs terminal commands in a loop - edits code, runs checks, validates work
5. **Result Review**: Shows answer and file diff; enables PR creation or follow-up questions

**Key Quote**: "Tasks run inside isolated, sandboxed cloud environments preloaded with the user's code, where Codex reads files, edits code, and runs commands and tests, returning logs and diffs for inspection." - [AI Agents Wiki](https://aiagents.wiki/agents/openai-codex)

---

## Architecture and Technical Foundation

### Model Foundation

The current Codex system is built on OpenAI's latest reasoning models, not the original Codex model:

- **codex-1** (May 2025): Initial model based on OpenAI's o3 reasoning model, optimized for software engineering through reinforcement learning on real coding tasks
- **GPT-5.3-Codex** (February 5, 2026): Major update
- **GPT-5.3-Codex-Spark** (February 12, 2026): Lower-latency variant for real-time interactive coding
- **GPT-5.4** (March 5, 2026): Latest model release
- **GPT-5.6 Family**: Current production models (Sol, Terra, Luna)
- **GPT-6 Astra**: Most capable model for complex work

**Source**: [OpenAI Developers Documentation](https://developers.openai.com/codex)

### Context Window

Codex supports a massive context window – up to ~192k tokens in its current form, enabling reasoning about large codebases.

### Multi-Agent Architecture

The system uses a sophisticated multi-agent architecture:

- **Threads can spawn sub-agent threads** or delegate specialized roles
- **Guardian sub-agent** for safety checks
- **Persistent state** managed by ThreadStore and StateDbHandle (SQLite-backed)
- **Multi-agent coordination** enables complex workflows (review agents, collaborative threads)

### Core Design Pattern

The architecture uses an **asynchronous submission/event queue (SQ/EQ) pattern**:

- **Submission**: Wraps an operation with metadata and trace context
- **Event**: Represents agent-generated notifications
- **Decoupled communication** between client interfaces and the core agent
- **Streaming, interruption, and cancellation** of operations

**Key Quote**: "The core design pattern is an asynchronous submission/event queue: user operations are submitted as Submission messages containing an Op, the agent processes them, and results stream back as EventMsg items." - [DeepWiki](https://deepwiki.com/openai/codex/1.3-architecture-overview)

### Unified Architecture

As of February 2026, OpenAI described a single architecture powering all client interfaces:

- CLI
- Visual Studio Code extension
- Web app
- macOS/Windows desktop apps
- Third-party IDE integrations

**Key Quote**: "The unified server keeps long-running sessions and approval requests consistent across client interfaces." - [Wikipedia](https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent))

---

## Integration with ChatGPT and Other OpenAI Products

### ChatGPT Integration

Codex is deeply integrated into the ChatGPT ecosystem:

- **Codex in ChatGPT**: Web/app interface for coding tasks
- **Codex IDE Extension**: Integrated into code editors
- **Codex CLI**: Command-line interface tool
- **Desktop App**: Native applications for Windows and macOS

### Multi-Platform Support

**Available Platforms**:
- **Web App**: Integrated into ChatGPT's web interface at chatgpt.com/codex
- **CLI**: Open-source terminal tool (Apache 2.0 license)
- **Desktop App**: Native applications for Windows and macOS
- **IDE Integrations**: Visual Studio Code, JetBrains, Xcode
- **GitHub Integration**: Available through GitHub Agent HQ

### Windows-Specific Features (March 2026)

- Native PowerShell support
- Windows-native agent sandbox with:
  - Restricted tokens
  - Filesystem permission controls (ACLs)

### Third-Party Integrations

**Major Integrations**:
1. **GitHub**: Agent HQ system, GitHub Mobile, VS Code for Copilot subscribers
2. **Figma**: Model Context Protocol (MCP) server for design-to-code workflow
3. **JetBrains**: IDE integration
4. **Xcode**: Apple development environment support

**GitHub Integration Details**:
- Developers can assign Codex to issues and pull requests
- Output can be compared with other agents (Claude, Copilot)
- Available in public preview for some Copilot subscribers

---

## Cloud-Based Code Execution Capabilities

### Parallel Cloud Environments

Codex Cloud provides isolated cloud environments for parallel task execution:

- Run multiple coding tasks simultaneously in isolated cloud environments
- Each task gets dedicated resources and continues running in the background
- Users can work on other tasks while long-running operations complete

**Source**: [OpenAI Codex Cloud Documentation](https://developers.openai.com/codex/cloud)

### Environment Configuration

**Custom Environments**:
- Configure dependencies, tools, variables, and setup steps per repository
- Create reusable environment templates for different projects
- Manage environment settings at chatgpt.com/codex/settings/environments

**Container Image**:
- **Universal image**: Default container with pre-installed languages, packages, and tools
- **Custom setup**: Support for setup scripts to install additional dependencies
- **Caching**: Container state cached for up to 12 hours; invalidates on setup/environment changes
- **Environment variables & secrets**: Encrypted storage; secrets only available during setup phase

### Integration Ecosystem

**Supported Platforms**:
- **GitHub**: Pull requests and issues (tag @Codex for review)
- **GitLab** (Beta): Merge requests and issues
- **Linear**: Issues and comments
- **Slack**: Channels and threads
- **Web interface**: Start and review tasks from anywhere

### Review Workflow

1. Inspect summaries and diffs before merging
2. Request follow-up changes
3. Open pull requests when work is ready
4. Code review integration with GitHub PRs

**Key Quote**: "Run tasks in isolated cloud environments, work in parallel, and start work from the web, GitHub, GitLab, Linear, or Slack." - [OpenAI Documentation](https://developers.openai.com/codex/cloud)

---

## Comparison with GitHub Copilot

### Nature of the Products

**OpenAI Codex**:
- Developed by OpenAI
- AI coding agent platform (not just a model)
- Powers GitHub Copilot and other coding assistants
- Trained on billions of lines of public code
- Can understand and generate code in dozens of programming languages
- Available as a comprehensive platform (CLI, web, desktop, IDE)

**GitHub Copilot**:
- Developed by GitHub (owned by Microsoft) in partnership with OpenAI
- AI pair programmer that helps write code faster
- Uses OpenAI Codex as its foundation
- Integrates directly into code editors (VS Code, Visual Studio, JetBrains, Neovim)
- Suggests whole lines or blocks of code as you type
- Offers chat functionality for code explanations and help

### Key Differences

1. **Nature**:
   - Codex is an AI agent platform
   - Copilot is a commercial product/service

2. **Accessibility**:
   - Codex: Available across multiple platforms (CLI, web, desktop, IDE)
   - Copilot: Available as a subscription for end-users in code editors

3. **Integration**:
   - Codex: Can be integrated into custom applications
   - Copilot: Comes pre-integrated into popular code editors

4. **Usage**:
   - Codex: More for autonomous task completion and software engineering
   - Copilot: For direct coding assistance and suggestions

### GitHub Integration

OpenAI Codex is now integrated with GitHub Copilot:

- **Integration Method**: Uses the Codex SDK
- **Powered By**: GitHub Copilot subscription infrastructure
- **Two Main Components**:
  1. **OpenAI Codex Coding Agent** - Cloud-based agent for task assignment
  2. **VS Code Extension** - Local IDE integration

**Source**: [GitHub Copilot Documentation](https://docs.github.com/en/copilot/concepts/agents/openai-codex)

### Competitive Landscape

**Main Competitors**: Claude Code, Cursor, GitHub Copilot, Cognition's Devin, Amazon Q Developer, Aider

**Differentiation**: OpenAI's first-party agent spanning cloud, CLI, IDE, and GitHub from a single ChatGPT account.

**Key Quote**: "Coding is one of artificial intelligence's most commercially successful uses and the market for coding agents had become a key battleground among AI companies." - [Wikipedia](https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent))

---

## OpenAI's Coding Strategy

### Product Consolidation

OpenAI has been consolidating its coding products:

- **July 2026**: Codex merged with ChatGPT desktop app into "superapp"
- Part of effort to simplify product lineup
- Response to growing competition from Anthropic

### Enterprise Agent Platform

**Strategic Positioning**: "OpenAI was positioning it as a broader enterprise agent platform that could eventually be used for tasks beyond software development." - [Wikipedia](https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent))

### Key Strategy Elements

1. **Multi-Agent Workflows**: Designed for multi-agent workflows using built-in worktrees and cloud environments
2. **Skills System**: Allows users to teach Codex their team's standards, workflows, and ways of working
3. **Background Operation**: Capable of "always-on background work" for routine tasks
4. **Quality Assurance**: Aims to raise baseline quality by providing more thorough designs, comprehensive testing, and high-signal code review

### Acquisition Strategy

- OpenAI agreed to acquire **Astral** (Python toolmaker)
- Strategic move to enhance Python development capabilities

**Key Quote**: "Codex moved beyond being just a coding model and became your Software Engineer teammate: connecting models, local tooling, and cloud to help developers tackle longer, more complex coding tasks." - [OpenAI Developers Blog](https://developers.openai.com/blog/openai-for-developers-2025)

---

## Technical Specifications and Limitations

### System Requirements

| Operating System | Details |
|------------------|---------|
| **macOS** | macOS 12+ (Apple Silicon/arm64 and x86_64) |
| **Linux** | Ubuntu 20.04+/Debian 10+ (x86_64 and arm64 via MUSL) |
| **Windows** | Windows 11 **via WSL2** (Recommended) or Native x86_64/arm64 |

**Resource Requirements**:
- **RAM**: 4-GB minimum (8-GB recommended)

### Development Requirements

- **Rust**: 1.85+ (standard for the `codex-rs` workspace)
- **Node.js**: ≥ 22 for workspace maintenance, ≥ 16 for CLI packaging
- **pnpm**: v10.34.5+ (used for workspace dependency management)

### Limitations & Considerations

#### Task Constraints
- Tasks run in sandboxed cloud environments
- Execution time: 1-30 minutes per task
- Requires user repository to be preloaded

#### Platform Limitations
- **Windows**: Requires WSL2 for full functionality (native Windows support is limited)
- **Resource Usage**: Minimum 4GB RAM recommended, 8GB for optimal performance

#### Security Considerations
- Runs in isolated environments
- Windows sandbox uses OS-level controls (restricted tokens, ACLs)
- Codex Security focuses on vulnerability detection

#### Model Limitations
- **Model Lock-in**: Tied to OpenAI models (not model-agnostic)
- **Vendor Claims**: Reliability and benchmark figures are vendor-reported
- **Human Review Required**: Not a hands-off autonomous engineer - requires human approval

### Deprecated Features

- **GPT-5.4 models**: Retiring August 31, 2026
- **GPT-5.2/5.3-codex**: Already deprecated
- **Chat Completions API**: Support being removed

**Key Quote**: "Codex is a supervised agent. It performs multi-step work autonomously inside its sandbox but surfaces diffs, logs, and pull requests for a developer to approve before changes land." - [AI Agents Wiki](https://aiagents.wiki/agents/openai-codex)

---

## Pricing and Availability

### Availability

- **ChatGPT Plus**: Access granted June 2025
- **ChatGPT Pro**: Earlier access to research previews
- **Enterprise**: Growing enterprise adoption
- **License**:
  - **CLI**: Apache License 2.0 (open-source)
  - **Desktop/Web**: Proprietary

### Pricing Structure

**Plan Tiers**:

| Plan | Price | Key Features |
|------|-------|--------------|
| **Free** | $0 | Explore Codex capabilities on quick coding tasks |
| **Go** | Varies | Lightweight coding tasks |
| **Plus** | Standard ChatGPT Plus | Codex on web/CLI/IDE/iOS, cloud integrations, GPT-5.6 models |
| **Pro 5x** | $100/month | 5x higher rate limits than Plus, Codex-Spark research preview |
| **Pro 20x** | $200/month | 20x higher limits, unlimited ChatGPT Voice |
| **API Key** | Usage-based | Automation in CI/CD, no cloud features |

### Token Pricing (Credits per 1M tokens)

| Model | Input | Cached Input | Output |
|-------|-------|--------------|--------|
| **GPT-6 Astra** | 250 | 25 | 1,250 |
| **GPT-5.6 Sol** | 100 | 10 | 500 |
| **GPT-5.6 Terra** | 50 | 5 | 300 |
| **GPT-5.6 Luna** | 5 | 0.5 | 30 |
| **GPT-5.5** | 125 | 12.50 | 750 |
| **GPT-5.4** | 62.50 | 6.250 | 375 |
| **GPT-5.4 mini** | 18.75 | 1.875 | 113 |

### Usage Limits (Messages per 5-hour period)

| Model | Plus | Pro 5x | Pro 20x |
|-------|------|--------|---------|
| **GPT-6 Astra** | 5-45 | 25-225 | 100-900 |
| **GPT-5.6 Sol** | 10-100 | 50-500 | 200-2,000 |
| **GPT-5.6 Terra** | 25-200 | 125-1,000 | 500-4,000 |
| **GPT-5.6 Luna** | 250-2,000 | 1,250-10,000 | 5,000-40,000 |

### GitHub Copilot Integration Pricing

- **OpenAI Codex Coding Agent**: Available for all paid Copilot plans
- **VS Code Extension "Sign in with Copilot"**: Only for Copilot Pro+ and Copilot Max subscribers
- **Credit System**: 1 AI credit = $0.01 USD
- **Free Tier**: Limited to 2000 completions and 50 chat requests

### Enterprise Features

- **SAML SSO, MFA, workspace management**
- **Cloud-managed config policies**
- **RBAC and custom roles**
- **SCIM, EKM, domain verification**
- **Retention and residency controls**
- **No training on business data by default**

**Key Quote**: "The tool had been in private beta since 2025... it worked by first building a threat model of a repository before looking for vulnerabilities and proposing fixes." - [Wikipedia](https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent))

---

## Conclusion

OpenAI Codex has evolved from a simple code generation model into a comprehensive AI coding agent platform. The system now spans multiple interfaces (CLI, web, desktop, IDE), supports sophisticated multi-agent workflows, and provides enterprise-grade security and compliance features.

The platform represents a significant shift from AI-assisted coding to AI-driven software engineering, where the AI agent takes responsibility for bounded tasks while maintaining human oversight through approval workflows and review processes.

With over 2 million weekly active users and deep integration into the GitHub and ChatGPT ecosystems, OpenAI Codex has established itself as a major player in the AI coding assistant market, competing directly with GitHub Copilot, Claude Code, and other emerging tools in this rapidly evolving space.

---

## Sources

1. [OpenAI Codex (AI agent) - Wikipedia](https://en.wikipedia.org/wiki/OpenAI_Codex_(AI_agent))
2. [Architecture Overview | openai/codex | DeepWiki](https://deepwiki.com/openai/codex/1.3-architecture-overview)
3. [OpenAI Codex: capabilities, pricing & alternatives | AI Agents Wiki](https://aiagents.wiki/agents/openai-codex)
4. [Codex for (almost) everything - OpenAI](https://openai.com/index/codex-for-almost-everything/)
5. [OpenAI for Developers in 2025](https://developers.openai.com/blog/openai-for-developers-2025)
6. [OpenAI Codex in 2025: A Comprehensive Evaluation](https://www.aicritique.org/us/2025/05/19/openai-codex-in-2025-a-comprehensive-evaluation/)
7. [Everything About Codex: The Complete Guide to OpenAI's Coding Agent](https://bhavishyapandit9.substack.com/p/everything-about-codex-the-complete)
8. [GitHub - openai/codex: Lightweight coding agent that runs in your terminal](https://github.com/openai/codex)
9. [Codex for Builders - Resource | OpenAI Academy](https://academy.openai.com/public/clubs/builders-etkn1/resources/codex-for-builders)
10. [OpenAI Codex Official Website](https://openai.com/codex/)
11. [OpenAI Codex CLI Documentation](https://developers.openai.com/codex/cli)
12. [OpenAI Codex Documentation Overview](https://developers.openai.com/codex)
13. [OpenAI Codex Cloud Documentation](https://developers.openai.com/codex/cloud)
14. [GitHub Copilot Documentation - OpenAI Codex](https://docs.github.com/en/copilot/concepts/agents/openai-codex)
15. [OpenAI Codex Security Repository](https://github.com/openai/codex-security)

---

*Document created on: October 15, 2026*
*Based on research conducted using DeepSeek Harness*