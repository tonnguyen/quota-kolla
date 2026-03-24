# Quota kolla

Quota kolla is a macOS menu bar app for keeping an eye on AI usage limits without opening each CLI or provider dashboard.

It renders compact provider widgets in the menu bar and shows a popup with every available usage window plus reset countdowns.

## What It Tracks

- Claude
  5h, 7d, Opus, Sonnet
- zAI
  5h and 30d
- Codex
  primary and secondary usage windows, shown as 5h and 7d in the UI

## Features

- Native macOS menu bar app built with Tauri 2
- Multiple widget display modes per provider: `bar`, `text`, `circle`
- Provider popup with:
  percentage used
  all available usage windows
  `Reset in ...` countdowns
- Preferences window for enabling/disabling providers
- Light/dark aware popup styling
- Background refresh of tray and popup data

## How It Works

Quota kolla reads local auth/config that your CLI tools already use, then calls provider APIs:

- Claude: macOS Keychain entry `Claude Code-credentials`, then Anthropic OAuth usage API
- zAI: `~/.ccs/glm.settings.json`, then Z.AI quota API
- Codex: `~/.codex/auth.json`, then the ChatGPT backend usage endpoint

## Requirements

- macOS
- Rust toolchain
- Node.js
- Bun or npm-compatible JS tooling
- Xcode Command Line Tools

For provider data to work, you also need to be logged into the corresponding CLI/provider.

## Install Dependencies

```bash
npm install
```

## Development

```bash
npm run dev
```

## Run Tests

From the repo root:

```bash
cd src-tauri
cargo test
```

Or in one line:

```bash
cargo test --manifest-path src-tauri/Cargo.toml
```

## Build

```bash
npm run build
```

The frontend assets are copied into `dist/` by `scripts/build-frontend.mjs`, and Tauri produces the app bundle during `tauri build`.

## CI / Releases

GitHub Actions builds desktop bundles automatically for:

- `main` pushes
  uploaded as workflow artifacts
- `v*` tags
  uploaded as workflow artifacts and attached to a draft GitHub Release

The workflow currently builds:

- macOS
- Windows

## Cut a Release

To create a release build through GitHub Actions, bump the app version first, commit it, then push a version tag.

Example:

```bash
git checkout main

# update version in:
# - package.json
# - src-tauri/Cargo.toml
# - src-tauri/tauri.conf.json

git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json
git commit -m "Bump version to 0.1.1"
git push

git tag v0.1.1
git push origin v0.1.1
```

That tag triggers the release workflow, which:

- builds macOS artifacts
- builds Windows artifacts
- creates or updates a draft GitHub Release
- attaches the generated bundles to that release

## Configuration

Quota kolla stores its config at:

- macOS: `~/Library/Application Support/quota-kolla/config.json`
- Linux: `~/.config/quota-kolla/config.json`
- Windows: `%APPDATA%\\quota-kolla\\config.json`

Example:

```json
{
  "version": 1,
  "providers": {
    "claude": { "visible": true, "mode": "bar" },
    "glm": { "visible": true, "mode": "bar" },
    "codex": { "visible": true, "mode": "bar" }
  }
}
```

## Repo Layout

- `src/`
  menu and preferences frontend
- `src-tauri/src/`
  Rust backend, tray rendering, provider fetching, config handling
- `build.sh`
  copies frontend files into `dist/` for Tauri

## Notes

- zAI uses `30d` for the long window in the popup.
- Codex depends on the local auth file format used by the Codex CLI.
- If a provider is unavailable or auth is missing, the popup shows an error row for that provider.
