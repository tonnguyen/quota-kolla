# macOS MenuBar Progress App Implementation Plan

**Date:** 2026-03-23
**Status:** Superseded by dropdown menu feature (2026-03-24)

## Overview

Build a macOS menubar app that displays Claude Code's 5-hour API usage as a circular progress ring.

## Tasks

### Task 1: Core Structure
- [x] Initialize Tauri 2.x project
- [x] Set up basic menubar icon
- [x] Create Rust backend for API calls
- [x] Implement Claude OAuth token reading from Keychain

### Task 2: Data Fetching
- [x] Implement `/api/oauth/usage` endpoint call
- [x] Parse JSON response for `five_hour.utilization`
- [x] Handle authentication errors gracefully
- [x] Add 5-minute refresh interval

### Task 3: UI Rendering
- [x] Create SVG circular progress component
- [x] Implement percentage calculation (0-100%)
- [x] Add color coding (blue for usage, gray for track)
- [x] Set 22×22 point icon size

### Task 4: Build & Deployment
- [x] Configure Tauri build settings
- [x] Create macOS app bundle
- [x] Test on target system
- [x] Verify menubar integration

### Task 5: Final Polish
- [x] Add error handling for network failures
- [x] Implement startup behavior
- [x] Add quit functionality
- [x] Document setup and usage

## Technical Details

### Stack
- Frontend: HTML/CSS/JS with Tauri 2.x
- Backend: Rust
- Platform: macOS

### API Endpoint
```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <oauth_token>
```

### Response Format
```json
{
  "five_hour": {
    "utilization": 0.45
  }
}
```

### Keychain Integration
- Service: `Claude Code-credentials`
- Account: `default`
- Token stored in macOS Keychain

## Update: Dropdown Menu Feature Added (2026-03-24)

This plan was extended with a dropdown menu feature. See [2026-03-24-dropdown-menu-implementation.md](./2026-03-24-dropdown-menu-implementation.md) for the implementation plan.

Original tasks 1-5 were completed. Task 6 (Final Polish) was superseded by the dropdown menu feature.
