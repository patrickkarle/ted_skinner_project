# FullIntel Agent - Work Resumption Manifest
## Post-Implementation System Documentation

**Version:** 0.1.1
**Last Updated:** 2025-12-02
**Status:** Production Ready (Cross-Platform)
**Repository:** https://github.com/patrickkarle/ted_skinner_project

---

## Table of Contents

1. [Project Overview](#1-project-overview)
2. [Architecture](#2-architecture)
3. [File Structure](#3-file-structure)
4. [Backend Components (Rust)](#4-backend-components-rust)
5. [Frontend Components (React)](#5-frontend-components-react)
6. [Research Manifests](#6-research-manifests)
7. [Build & Development](#7-build--development)
8. [CI/CD & Releases](#8-cicd--releases)
9. [Testing](#9-testing)
10. [Integration Points](#10-integration-points)
11. [Database Schema](#11-database-schema)
12. [Current State](#12-current-state)
13. [Known Issues & Technical Debt](#13-known-issues--technical-debt)
14. [Future Development Areas](#14-future-development-areas)
15. [Quick Start Guide](#15-quick-start-guide)

---

## 1. Project Overview

### What is FullIntel Agent?

A cross-platform desktop application for AI-powered research workflows. Users can:
- Execute structured multi-phase research protocols on companies, industries, and technologies
- Chat directly with LLMs without research protocols
- Manage research sessions with full history and resume capability
- Create and edit custom research manifests
- Export results in multiple formats

### Key Capabilities

| Feature | Description |
|---------|-------------|
| **Multi-Provider AI** | Anthropic Claude, OpenAI GPT, Google Gemini, DeepSeek + custom providers |
| **Dual-Mode Operation** | Chat mode (direct LLM) + Research mode (structured workflows) |
| **7 Research Protocols** | Pre-built manifests for different analysis types |
| **Session Management** | SQLite persistence, resume, history, projects |
| **Cross-Platform** | Windows, macOS (Intel + ARM), Linux |
| **Security** | Local-first, encrypted API keys (AES-256-GCM), Argon2id passwords |

### Technology Stack

| Layer | Technology |
|-------|------------|
| **Frontend** | React 18 + TypeScript + Vite |
| **Backend** | Rust + Tauri v2 |
| **Database** | SQLite (rusqlite) |
| **Encryption** | AES-256-GCM (API keys), Argon2id (passwords) |
| **Build** | Cargo (Rust) + npm (Node.js) |
| **CI/CD** | GitHub Actions |

---

## 2. Architecture

### High-Level Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     FullIntel Agent                              │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    React Frontend                        │    │
│  │  ┌──────────┐  ┌──────────────┐  ┌─────────────────┐   │    │
│  │  │ App.tsx  │  │ AuthScreen   │  │ SettingsPanel   │   │    │
│  │  │ (Main)   │  │ (Login)      │  │ (API Keys)      │   │    │
│  │  └──────────┘  └──────────────┘  └─────────────────┘   │    │
│  │  ┌──────────────────────────────────────────────────┐  │    │
│  │  │              ManifestEditor                       │  │    │
│  │  │              (YAML editing)                       │  │    │
│  │  └──────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│                    Tauri IPC (invoke)                           │
│                              │                                   │
│  ┌─────────────────────────────────────────────────────────┐    │
│  │                    Rust Backend                          │    │
│  │  ┌──────────┐  ┌──────────┐  ┌──────────┐  ┌────────┐  │    │
│  │  │ main.rs  │  │ auth.rs  │  │ llm.rs   │  │agent.rs│  │    │
│  │  │(Commands)│  │(Security)│  │(AI APIs) │  │(Engine)│  │    │
│  │  └──────────┘  └──────────┘  └──────────┘  └────────┘  │    │
│  │  ┌──────────────────────────────────────────────────┐  │    │
│  │  │              manifest.rs (YAML parsing)           │  │    │
│  │  └──────────────────────────────────────────────────┘  │    │
│  └─────────────────────────────────────────────────────────┘    │
│                              │                                   │
│           ┌──────────────────┼──────────────────┐               │
│           │                  │                  │               │
│           ▼                  ▼                  ▼               │
│    ┌───────────┐      ┌───────────┐      ┌───────────┐         │
│    │  SQLite   │      │  AI APIs  │      │ Manifests │         │
│    │ (users.db)│      │ (HTTP/S)  │      │  (YAML)   │         │
│    └───────────┘      └───────────┘      └───────────┘         │
└─────────────────────────────────────────────────────────────────┘
```

### Data Flow

1. **User Authentication**: React → `login`/`register` command → auth.rs → SQLite
2. **Research Workflow**: React → `run_research` command → agent.rs → llm.rs → AI API
3. **Settings**: React → `save_api_key` command → auth.rs → Encrypted SQLite
4. **Manifests**: React → `list_manifests` command → manifest.rs → Filesystem YAML

### Tauri IPC Commands (main.rs)

| Command | Purpose | Module |
|---------|---------|--------|
| `login` | User authentication | auth.rs |
| `register` | Create user account | auth.rs |
| `save_api_key` | Store encrypted API key | auth.rs |
| `get_api_key` | Retrieve decrypted API key | auth.rs |
| `run_research` | Execute research workflow | agent.rs |
| `run_chat` | Direct LLM chat | llm.rs |
| `list_manifests` | Get available protocols | manifest.rs |
| `get_manifest_content` | Read manifest YAML | manifest.rs |
| `save_manifest` | Save custom manifest | manifest.rs |
| `create_session` | New research session | auth.rs |
| `save_phase_output` | Store phase results | auth.rs |
| `get_session_history` | List past sessions | auth.rs |
| `archive_session` | Archive a session | auth.rs |

---

## 3. File Structure

```
ted_skinner_project/
├── src/                          # React Frontend
│   ├── App.tsx                   # Main application (170KB, ~4500 lines)
│   ├── App.css                   # Application styles (44KB)
│   ├── main.tsx                  # React entry point
│   ├── design-tokens.json        # UI design system
│   ├── components/
│   │   ├── AuthScreen.tsx        # Login/register UI
│   │   ├── AuthScreen.css
│   │   ├── SettingsPanel.tsx     # API key management, custom providers
│   │   ├── SettingsPanel.css
│   │   ├── ManifestEditor.tsx    # YAML editor for manifests
│   │   └── ManifestEditor.css
│   └── assets/
│       └── fullintel_logo.jpg
│
├── src-tauri/                    # Rust Backend
│   ├── src/
│   │   ├── main.rs              # Tauri commands & app setup (66KB)
│   │   ├── agent.rs             # Research workflow engine (19KB)
│   │   ├── auth.rs              # Auth & database (81KB)
│   │   ├── llm.rs               # LLM API integration (109KB)
│   │   ├── manifest.rs          # YAML manifest parsing (4KB)
│   │   └── lib.rs               # Module exports
│   ├── tests/
│   │   ├── battery1_unit_strategic.rs
│   │   ├── battery2_integration_strategic.rs
│   │   ├── battery3_system_strategic.rs
│   │   ├── unit_agent.rs
│   │   └── integration_e2e.rs
│   ├── icons/                   # App icons (all platforms)
│   ├── Cargo.toml               # Rust dependencies
│   └── tauri.conf.json          # Tauri configuration
│
├── manifests/                   # Research protocols (YAML)
│   ├── fullintel_process_manifest.yaml
│   ├── Deep-Dive-Industry-Protocol.yaml
│   ├── frontier-tech-protocol.yaml
│   ├── Inductive-Company-Protocol.yaml
│   ├── Intellectual-capital-protocol.yaml
│   ├── value-chain-protocol.yaml
│   └── venture-momentum-protocol.yaml
│
├── scripts/
│   └── generate-icons.mjs       # Icon generation (RGBA format)
│
├── .github/workflows/
│   ├── build.yml                # Cross-platform build & release
│   └── test.yml                 # Automated tests
│
├── docs/                        # Additional documentation
├── briefs-generated/            # Generated research briefs
├── session-handoffs/            # Previous handoff documents
├── FullIntel-Agent-Delivery/    # Distribution package (gitignored)
│
├── package.json                 # Node.js dependencies
├── tsconfig.json                # TypeScript config
├── vite.config.ts               # Vite bundler config
├── index.html                   # HTML entry point
└── README.md                    # User documentation
```

---

## 4. Backend Components (Rust)

### main.rs - Command Hub (~2100 lines)

The central command registry for Tauri IPC. Key responsibilities:
- Register all Tauri commands
- Handle window setup
- Manage app state
- Route commands to appropriate modules

**Key Tauri Commands:**
```rust
#[tauri::command]
async fn login(username: &str, password: &str) -> Result<LoginResponse, String>

#[tauri::command]
async fn run_research(
    provider: &str,
    model: &str,
    topic: &str,
    manifest_id: &str,
    session_id: i64,
    window: Window
) -> Result<String, String>

#[tauri::command]
async fn run_chat(
    provider: &str,
    model: &str,
    messages: Vec<ChatMessage>,
    window: Window
) -> Result<String, String>
```

### auth.rs - Security & Persistence (~2400 lines)

Handles all authentication and database operations.

**Security Features:**
- Password hashing: Argon2id
- API key encryption: AES-256-GCM
- User isolation: Separate data per user
- Session management: Full CRUD

**Database Tables:**
```sql
users (id, username, password_hash, created_at)
api_keys (id, user_id, provider, encrypted_key, created_at)
sessions (id, user_id, topic, manifest_id, status, created_at)
phase_outputs (id, session_id, phase_id, output, created_at)
projects (id, user_id, name, created_at)
custom_providers (id, user_id, name, base_url, model_id, api_key)
```

### llm.rs - AI Provider Integration (~3200 lines)

Supports multiple AI providers with streaming responses.

**Supported Providers:**

| Provider | Models | Streaming |
|----------|--------|-----------|
| Anthropic | Claude Opus 4, Sonnet 4.5, Haiku 3.5 | Yes |
| OpenAI | GPT-4o, GPT-4 Turbo, GPT-3.5, o1-preview, o3-mini | Yes |
| Google | Gemini 1.5 Pro, Gemini 1.5 Flash | Yes |
| DeepSeek | DeepSeek Chat, DeepSeek Coder | Yes |
| Custom | Any OpenAI-compatible API | Yes |

**Key Structs:**
```rust
pub struct LlmClient {
    provider: Provider,
    api_key: String,
    model: String,
}

pub enum Provider {
    Anthropic,
    OpenAI,
    Google,
    DeepSeek,
    Custom { base_url: String },
}
```

### agent.rs - Research Engine (~550 lines)

Orchestrates multi-phase research workflows.

**Workflow:**
1. Parse manifest YAML
2. For each phase:
   - Build system prompt with instructions
   - Inject current date for research freshness
   - Call LLM with context from previous phases
   - Stream response to frontend
   - Store phase output
3. Return complete research brief

**Key Function:**
```rust
pub async fn run_research_workflow(
    client: &LlmClient,
    manifest: &Manifest,
    topic: &str,
    session_id: i64,
    window: &Window,
) -> Result<String, AgentError>
```

### manifest.rs - Protocol Parser (~120 lines)

Parses YAML research manifests into structured data.

**Manifest Structure:**
```yaml
manifest:
  id: "Protocol-ID"
  version: "1.0.0"
  name: "Protocol Name"
  description: "What this analyzes"

phases:
  - id: "PHASE-01"
    name: "Phase Name"
    instructions: |
      Detailed AI instructions...
    output_schema: "SchemaName"
    dependencies: []  # Previous phases to include

quality_gates:
  - phase: "PHASE-01"
    check: "Validation criteria"
    fail_action: "RETRY"
```

---

## 5. Frontend Components (React)

### App.tsx - Main Application (~4500 lines)

The monolithic main component handling:
- Application state management
- UI rendering for all modes
- Tauri command invocation
- Real-time streaming display
- Session management UI

**Key State:**
```typescript
interface AppState {
  isAuthenticated: boolean;
  currentUser: User | null;
  selectedProvider: string;
  selectedModel: string;
  selectedManifest: string;
  sessions: Session[];
  currentSession: Session | null;
  messages: ChatMessage[];
  isResearchMode: boolean;
  isRunning: boolean;
}
```

### AuthScreen.tsx - Login/Register (~200 lines)

Handles user authentication flow:
- Login form
- Registration form
- Error display
- Session persistence

### SettingsPanel.tsx - Configuration (~700 lines)

Manages:
- API key entry per provider
- Custom provider configuration
- User preferences
- Provider selection

### ManifestEditor.tsx - YAML Editor (~450 lines)

Features:
- Syntax-highlighted YAML editing
- Validation with error display
- Save/Load functionality
- Phase preview

---

## 6. Research Manifests

### Available Protocols

| Manifest | File | Purpose |
|----------|------|---------|
| Fullintel Process | `fullintel_process_manifest.yaml` | General company research |
| Deep Dive Industry | `Deep-Dive-Industry-Protocol.yaml` | Industry sector analysis |
| Frontier Tech | `frontier-tech-protocol.yaml` | Emerging technology research |
| Inductive Company | `Inductive-Company-Protocol.yaml` | Bottom-up company analysis |
| Intellectual Capital | `Intellectual-capital-protocol.yaml` | Talent/human capital mapping |
| Value Chain | `value-chain-protocol.yaml` | Supply chain analysis |
| Venture Momentum | `venture-momentum-protocol.yaml` | Startup/investment research |

### Creating Custom Manifests

1. Use the in-app Manifest Editor
2. Or create YAML file in `manifests/` directory
3. Follow the schema structure
4. Restart app or use "Load" in editor

---

## 7. Build & Development

### Prerequisites

| Tool | Version | Purpose |
|------|---------|---------|
| Node.js | 18+ | Frontend build |
| Rust | 1.70+ | Backend compilation |
| Tauri CLI | 2.0+ | App bundling |

**Platform-Specific:**

| Platform | Additional Requirements |
|----------|------------------------|
| Windows | Visual Studio Build Tools (C++ workload), WebView2 |
| macOS | Xcode Command Line Tools |
| Linux | `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `librsvg2-dev` |

### Development Commands

```bash
# Install dependencies
npm install

# Run in development mode (hot reload)
npm run tauri dev

# Build for production (current platform)
npm run tauri build

# Run Rust tests
cd src-tauri && cargo test

# Regenerate icons
node scripts/generate-icons.mjs
```

### Build Outputs

| Platform | Output Location |
|----------|-----------------|
| Windows | `src-tauri/target/release/bundle/nsis/*.exe` |
| Windows | `src-tauri/target/release/bundle/msi/*.msi` |
| macOS | `src-tauri/target/release/bundle/dmg/*.dmg` |
| Linux | `src-tauri/target/release/bundle/appimage/*.AppImage` |
| Linux | `src-tauri/target/release/bundle/deb/*.deb` |

---

## 8. CI/CD & Releases

### GitHub Actions Workflows

**`.github/workflows/build.yml`** - Cross-platform build:
- Triggers on: Version tags (`v*`) or manual dispatch
- Builds on: ubuntu-22.04, macos-latest (x2), windows-latest
- Creates: Draft release with all 6 binaries

**`.github/workflows/test.yml`** - Automated tests:
- Triggers on: Push to master
- Runs: `cargo test` for all test batteries

### Creating a Release

```bash
# Bump version in these files:
# - src-tauri/tauri.conf.json
# - src-tauri/Cargo.toml

# Commit changes
git add -A && git commit -m "chore: bump version to X.Y.Z"

# Create and push tag
git tag -a vX.Y.Z -m "vX.Y.Z - Release description"
git push origin master --tags

# CI will build all platforms and create draft release
# Edit release notes at: https://github.com/patrickkarle/ted_skinner_project/releases
# Click "Publish release"
```

### Current Release: v0.1.1

| Platform | File |
|----------|------|
| Windows | `fullintel-agent_0.1.0_x64-setup.exe` |
| Windows | `fullintel-agent_0.1.0_x64_en-US.msi` |
| macOS Intel | `fullintel-agent_0.1.0_x64.dmg` |
| macOS ARM | `fullintel-agent_0.1.0_aarch64.dmg` |
| Linux | `fullintel-agent_0.1.0_amd64.AppImage` |
| Linux | `fullintel-agent_0.1.0_amd64.deb` |

---

## 9. Testing

### Test Structure

```
src-tauri/tests/
├── battery1_unit_strategic.rs    # Unit tests
├── battery2_integration_strategic.rs  # Integration tests
├── battery3_system_strategic.rs  # System tests
├── unit_agent.rs                 # Agent-specific tests
└── integration_e2e.rs            # End-to-end tests
```

### Running Tests

```bash
cd src-tauri

# Run all tests
cargo test

# Run specific battery
cargo test battery1  # Unit tests
cargo test battery2  # Integration tests
cargo test battery3  # System tests

# Run with output
cargo test -- --nocapture
```

### Test Coverage Areas

- **Authentication**: Login, register, password hashing
- **Encryption**: API key encrypt/decrypt
- **LLM Clients**: Provider detection, model validation
- **Manifests**: YAML parsing, validation
- **Sessions**: CRUD operations, persistence

---

## 10. Integration Points

### LLM API Endpoints

| Provider | Base URL | Auth Header |
|----------|----------|-------------|
| Anthropic | `https://api.anthropic.com/v1/messages` | `x-api-key` |
| OpenAI | `https://api.openai.com/v1/chat/completions` | `Authorization: Bearer` |
| Google | `https://generativelanguage.googleapis.com/v1beta/models/{model}:streamGenerateContent` | `?key=` |
| DeepSeek | `https://api.deepseek.com/chat/completions` | `Authorization: Bearer` |

### Custom Provider Configuration

Users can add OpenAI-compatible APIs:
```json
{
  "name": "Local Ollama",
  "base_url": "http://localhost:11434/v1",
  "model_id": "llama3.2",
  "api_key": ""  // Optional
}
```

### Frontend-Backend Communication

All communication via Tauri's `invoke()`:
```typescript
// Frontend
const result = await invoke('run_research', {
  provider: 'anthropic',
  model: 'claude-sonnet-4-5-20250929',
  topic: 'Acme Corp',
  manifestId: 'fullintel-process',
  sessionId: 123
});

// Backend (main.rs)
#[tauri::command]
async fn run_research(...) -> Result<String, String>
```

### Event Streaming

Real-time updates via Tauri events:
```rust
// Backend
window.emit("research_update", payload)?;

// Frontend
listen('research_update', (event) => {
  setOutput(prev => prev + event.payload);
});
```

---

## 11. Database Schema

### Location

| Platform | Path |
|----------|------|
| Windows | `%APPDATA%/com.fullintel.agent/users.db` |
| macOS | `~/Library/Application Support/com.fullintel.agent/users.db` |
| Linux | `~/.local/share/com.fullintel.agent/users.db` |

### Schema

```sql
-- User accounts
CREATE TABLE users (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    username TEXT UNIQUE NOT NULL,
    password_hash TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP
);

-- Encrypted API keys
CREATE TABLE api_keys (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    provider TEXT NOT NULL,
    encrypted_key TEXT NOT NULL,
    nonce TEXT NOT NULL,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id),
    UNIQUE(user_id, provider)
);

-- Research sessions
CREATE TABLE sessions (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    topic TEXT NOT NULL,
    manifest_id TEXT NOT NULL,
    status TEXT DEFAULT 'in_progress',
    project_id INTEGER,
    archived INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Phase outputs
CREATE TABLE phase_outputs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id INTEGER NOT NULL,
    phase_id TEXT NOT NULL,
    phase_name TEXT,
    output TEXT NOT NULL,
    prompts TEXT,  -- JSON: {system_prompt, user_prompt}
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (session_id) REFERENCES sessions(id)
);

-- Projects for grouping sessions
CREATE TABLE projects (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    archived INTEGER DEFAULT 0,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);

-- Custom AI providers
CREATE TABLE custom_providers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    user_id INTEGER NOT NULL,
    name TEXT NOT NULL,
    base_url TEXT NOT NULL,
    model_id TEXT NOT NULL,
    api_key TEXT,
    created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id)
);
```

---

## 12. Current State

### What's Working (v0.1.1)

| Feature | Status |
|---------|--------|
| User authentication | ✅ Complete |
| API key encryption | ✅ Complete |
| Multi-provider LLM support | ✅ Complete |
| Research workflow execution | ✅ Complete |
| Direct chat mode | ✅ Complete |
| Session persistence | ✅ Complete |
| Session resume | ✅ Complete |
| Manifest editor | ✅ Complete |
| Custom providers | ✅ Complete |
| Project organization | ✅ Complete |
| Archive system | ✅ Complete |
| Cross-platform builds | ✅ Complete |
| Streaming responses | ✅ Complete |
| Export (clipboard, markdown) | ✅ Complete |

### Recent Changes (v0.1.1)

1. Cross-platform support (Windows, macOS, Linux)
2. GitHub Actions CI/CD pipeline
3. Icon RGBA format fix for Tauri
4. App.tsx file casing fix for case-sensitive filesystems
5. Available Downloads documentation

---

## 13. Known Issues & Technical Debt

### Issues

| Issue | Severity | Notes |
|-------|----------|-------|
| Large App.tsx file | Medium | 4500 lines, could be split into components |
| Version mismatch | Low | package.json says 0.0.0, should sync with 0.1.1 |
| Dependabot alerts | Low | 2 moderate vulnerabilities to address |
| macOS not notarized | Medium | Users must right-click → Open on first launch |

### Technical Debt

1. **Component Refactoring**: App.tsx should be split into smaller components
2. **State Management**: Consider adding Zustand or similar for complex state
3. **Error Handling**: More granular error types and user-friendly messages
4. **Logging**: Add structured logging for debugging
5. **Offline Mode**: Handle network failures gracefully

---

## 14. Future Development Areas

### Potential Enhancements

| Feature | Priority | Complexity |
|---------|----------|------------|
| PDF export | High | Medium |
| Research templates | Medium | Low |
| Batch research | Medium | High |
| Research history search | Medium | Medium |
| Keyboard shortcuts | Low | Low |
| Dark mode | Low | Medium |
| Multiple windows | Low | High |
| Plugin system | Low | High |
| Local LLM support (Ollama UI) | Medium | Medium |

### Architecture Improvements

1. **Modularize Frontend**: Split App.tsx into feature modules
2. **Add State Management**: Implement Zustand or Redux
3. **Improve Type Safety**: Stricter TypeScript configuration
4. **Add E2E Tests**: Playwright or Cypress for UI testing
5. **macOS Code Signing**: Get Apple Developer certificate for notarization

---

## 15. Quick Start Guide

### For New Developers

```bash
# 1. Clone repository
git clone https://github.com/patrickkarle/ted_skinner_project.git
cd ted_skinner_project

# 2. Install dependencies
npm install

# 3. Run in development mode
npm run tauri dev

# 4. Make changes, test, commit
git checkout -b feature/my-feature
# ... make changes ...
npm run tauri dev  # Test
cd src-tauri && cargo test  # Run tests
git add -A && git commit -m "feat: my feature"
git push origin feature/my-feature
```

### Key Entry Points

| Task | Start Here |
|------|------------|
| Add new Tauri command | `src-tauri/src/main.rs` |
| Modify LLM integration | `src-tauri/src/llm.rs` |
| Change authentication | `src-tauri/src/auth.rs` |
| Update research engine | `src-tauri/src/agent.rs` |
| Modify UI | `src/App.tsx` or `src/components/` |
| Add research protocol | `manifests/*.yaml` |
| Update build/CI | `.github/workflows/` |

### Environment Setup

1. Get API keys from providers (see README.md)
2. Run app: `npm run tauri dev`
3. Create account in app
4. Go to Settings → Enter API key
5. Start researching!

### Debugging Tips

```bash
# View Rust backend logs
RUST_LOG=debug npm run tauri dev

# Check Tauri IPC
# Open DevTools in app: Right-click → Inspect

# Test specific Rust module
cd src-tauri
cargo test llm --nocapture

# Rebuild frontend only
npm run build

# Clean Rust build
cd src-tauri && cargo clean
```

---

## Document History

| Date | Version | Changes |
|------|---------|---------|
| 2025-12-02 | 1.0 | Initial manifest creation |

---

**End of Work Resumption Manifest**
