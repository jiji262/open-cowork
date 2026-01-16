<div align="center">

# open-cowork

[![Version](https://img.shields.io/badge/version-0.1.0-blue.svg)](https://github.com/jiji262/open-cowork/releases)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey.svg)](https://github.com/jiji262/open-cowork/releases)

</div>

open-cowork is an open-source desktop AI collaboration workspace built with Tauri and React. It brings agent-style workflows to the desktop with explicit permissions, streaming output, and session-based context for everyday tasks and development work.

## Overview

- Local-first desktop agent environment with visible tool execution
- Session-based workflows with working directory binding
- Streaming output and Markdown rendering for long-running tasks
- Provider-agnostic support for Anthropic and OpenAI APIs

## Demo

- Video: `demo/cowork.mp4`

## Features

- Desktop-first UI with low overhead
- Multi-provider support (Anthropic/OpenAI)
- Token streaming with Markdown rendering
- Tool execution with approval modes
- Session management with working directory binding

## Download

- Releases: https://github.com/jiji262/open-cowork/releases

## Quick Start (Source)

### Requirements

- Node.js 18+ or Bun
- Rust toolchain (cargo)
- Tauri CLI (cargo tauri)

### Install and Run

```bash
git clone https://github.com/jiji262/open-cowork.git
cd open-cowork

bun install
bun run tauri:dev
```

### Build (Binary)

```bash
bun run tauri:build
```

## Provider Setup

Open the settings panel and configure API keys and model names:

- Anthropic: `claude-sonnet-4-5-20250929`
- OpenAI: `gpt-4o`, `gpt-4.1`

Settings are stored locally on the machine.

## Security Model

- Tool calls are visible in the UI
- Approval modes: auto-run or ask first
- Confirm before destructive actions

## Tech Stack

| Layer | Tech |
| --- | --- |
| Desktop | Tauri 2.x |
| UI | React 19 + Tailwind CSS 4 |
| State | Zustand |
| Backend | Rust + Tauri commands |
| AI | @anthropic-ai/claude-agent-sdk + provider APIs |
| Build | Vite + Cargo |

## Roadmap

- Persisted session history
- More provider integrations
- Stronger sandboxing and policy control

## Contributing

Pull requests and issues are welcome:

1. Fork the repo
2. Create a feature branch
3. Commit your changes
4. Open a PR

## License

MIT
