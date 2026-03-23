# MenuBar Progress App

A macOS menu bar app displaying Claude's 5-hour usage as a circular progress ring.

## Requirements

- macOS
- Rust
- Node.js 18+
- Xcode Command Line Tools
- Claude Code logged in (OAuth token stored in macOS Keychain)

## Build

```bash
npm install
npm run tauri build
```

App bundle: `src-tauri/target/release/bundle/macos/MenuBar Progress.app`

## Development

```bash
npm run tauri dev
```

## How It Works

1. Reads the Claude OAuth token from macOS Keychain (`Claude Code-credentials`)
2. Calls `https://api.anthropic.com/api/oauth/usage` → `five_hour.utilization`
3. Renders a circular progress ring SVG with the usage percentage
4. Refreshes every 5 minutes

## Design

- 22×22 point circular progress ring
- Blue (`#007AFF`) arc proportional to Claude 5h usage
- Gray background track (`#D1D1D6`)
- Built with Tauri 2.x (Rust)

## Usage

```bash
open "src-tauri/target/release/bundle/macos/MenuBar Progress.app"
```

The icon appears in the menu bar. To quit: `pkill -f "MenuBar Progress"`
