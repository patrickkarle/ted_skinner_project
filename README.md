# FullIntel Agent

A powerful desktop application for AI-powered research workflows and intelligent conversations. Execute multi-phase research protocols, engage in LLM chat, and generate comprehensive executive briefs on companies, industries, and emerging technologies.

![Version](https://img.shields.io/badge/version-0.1.1-blue)
![Platform](https://img.shields.io/badge/platform-Windows%20|%20macOS%20|%20Linux-lightgrey)

## Features

### 🤖 Multi-Provider AI Support
- **4 Built-in Providers** - Anthropic Claude, OpenAI GPT, Google Gemini, DeepSeek
- **Custom Provider Support** - Add your own API endpoints (e.g., local Ollama, custom OpenAI-compatible APIs)
- **Per-Provider API Keys** - Configure different keys for each provider
- **Model Selection** - Choose from 12+ models across providers

### 💬 Dual-Mode Operation
- **Chat Mode** - Direct LLM conversations without requiring research workflows
- **Research Mode** - Execute structured multi-phase research protocols
- **Seamless Switching** - Toggle between modes with a single click
- **Post-Research Followup** - Continue conversations after research completes

### 📋 Manifest System
- **7 Pre-configured Protocols** - Ready-to-use research workflows for different analysis types
- **Manifest Editor** - Full-featured YAML editor with syntax validation
- **Create Custom Manifests** - Design your own research workflows from scratch
- **Load External Manifests** - Import YAML files from anywhere on your system
- **Live Phase Preview** - See validated phases before running
- **Manifest Management** - Rename, remove, and organize your manifest library

### 📊 Research Session Management
- **Full Session Persistence** - All sessions saved to local SQLite database
- **Session History** - Browse all past research sessions with search
- **Session Resume** - Continue incomplete research from where you left off
- **Phase-Level Storage** - Each phase output saved individually with timestamps
- **Project Organization** - Group related sessions into named projects
- **Archive System** - Archive and restore both projects and sessions

### ✏️ Prompt Editing & Relaunch
- **View Prompts** - See exactly what system/user prompts were sent to the LLM
- **Edit Prompts** - Modify prompts before re-running a phase
- **Phase Relaunch** - Re-run any individual phase with original or modified prompts
- **Prompt History** - All prompts preserved for reproducibility

### 🔄 Real-Time Features
- **Streaming Output** - Watch AI responses appear token-by-token
- **Live Phase Status** - Visual progress indicators for each phase
- **Activity Logs** - Detailed execution logs with timing information
- **Abort Support** - Cancel running operations at any time

### 📤 Export & Output
- **Copy to Clipboard** - One-click copy of full research output
- **Markdown Export** - Save results as formatted Markdown files
- **Print Support** - Print-optimized views for physical copies
- **Per-Phase Export** - Export individual phase outputs separately

### 🔐 Security & Privacy
- **Local-First Design** - All data stored locally on your machine
- **Encrypted Credentials** - API keys and passwords securely stored
- **User Accounts** - Multi-user support with separate data isolation
- **No External Telemetry** - Only API calls to your selected LLM provider

## Installation

### Available Downloads

All binaries are available on the [GitHub Releases](../../releases) page:

| Platform | File | Type |
|----------|------|------|
| **Windows** | `fullintel-agent_0.1.1_x64-setup.exe` | NSIS installer (recommended) |
| **Windows** | `fullintel-agent_0.1.1_x64_en-US.msi` | MSI installer (enterprise) |
| **macOS Intel** | `fullintel-agent_0.1.1_x64.dmg` | Disk image |
| **macOS Apple Silicon** | `fullintel-agent_0.1.1_aarch64.dmg` | Disk image (M1/M2/M3) |
| **Linux** | `fullintel-agent_0.1.1_amd64.AppImage` | Universal (any distro) |
| **Linux** | `fullintel-agent_0.1.1_amd64.deb` | Debian/Ubuntu package |

### Windows

1. Download `fullintel-agent_0.1.1_x64-setup.exe` from [Releases](../../releases)
2. Run the installer and follow the wizard
3. Launch "FullIntel Agent" from the Start Menu

**Alternative:** Download the `.msi` installer for enterprise deployment.

### macOS

#### Intel Macs (x64)
1. Download `fullintel-agent_0.1.1_x64.dmg` from [Releases](../../releases)
2. Open the DMG and drag the app to Applications
3. Launch from Applications folder

#### Apple Silicon (M1/M2/M3)
1. Download `fullintel-agent_0.1.1_aarch64.dmg` from [Releases](../../releases)
2. Open the DMG and drag the app to Applications
3. Launch from Applications folder

**Note:** On first launch, you may need to right-click → Open to bypass Gatekeeper since the app is not notarized.

### Linux

#### AppImage (Universal)
1. Download `fullintel-agent_0.1.1_amd64.AppImage` from [Releases](../../releases)
2. Make it executable: `chmod +x fullintel-agent_*.AppImage`
3. Run it: `./fullintel-agent_*.AppImage`

#### Debian/Ubuntu (.deb)
1. Download `fullintel-agent_0.1.1_amd64.deb` from [Releases](../../releases)
2. Install: `sudo dpkg -i fullintel-agent_*.deb`
3. Launch from your application menu or run `fullintel-agent`

### System Requirements

| Platform | Requirements |
|----------|-------------|
| **Windows** | Windows 10/11 (64-bit), WebView2 Runtime |
| **macOS** | macOS 10.15 (Catalina) or later |
| **Linux** | GTK 3, WebKitGTK 4.1, glibc 2.31+ |

### API Key Requirement

You'll need an API key from at least one supported provider:
- [Anthropic Claude](https://console.anthropic.com)
- [OpenAI GPT](https://platform.openai.com)
- [Google Gemini](https://aistudio.google.com)
- [DeepSeek](https://platform.deepseek.com)

## Quick Start

### First Time Setup
1. **Create Account** - Set up a local user account on first launch
2. **Configure API Key** - Go to Settings (gear icon) and enter your API key for any supported provider

### Chat Mode (Default)
1. **Select Model** - Choose your AI provider and model from the dropdowns
2. **Type Message** - Enter your question or prompt in the input field
3. **Send** - Press Enter or click Send to chat with the AI
4. **Continue Conversation** - Follow up with additional questions

### Research Mode
1. **Switch Mode** - Click "Research" toggle (next to Chat)
2. **Select Manifest** - Choose a research protocol from the dropdown
3. **Enter Topic** - Type a company name or research subject
4. **Run Research** - Click "Run" and watch results stream in phase by phase
5. **Follow Up** - Ask questions about the research after it completes

### Session Management
1. **View History** - Browse past sessions in the left sidebar
2. **Resume Session** - Click any incomplete session to resume from last phase
3. **Edit & Relaunch** - View prompts for any phase, edit them, and re-run
4. **Organize** - Create projects to group related research sessions

## Research Manifests

The application includes 7 pre-configured research protocols:

| Manifest | Purpose |
|----------|---------|
| **Fullintel Process** | General company research and competitive analysis |
| **Deep Dive Industry** | Comprehensive industry sector analysis |
| **Frontier Tech** | Emerging technology and innovation research |
| **Inductive Company** | Bottom-up company analysis from public signals |
| **Intellectual Capital** | Talent mapping and human capital analysis |
| **Value Chain** | Supply chain and value network analysis |
| **Venture Momentum** | Startup and investment landscape research |

### Manifest Structure

Each manifest defines a multi-phase research workflow in YAML format:

```yaml
manifest:
  id: "Protocol-ID"
  version: "1.0.0"
  name: "Protocol Name"
  description: "What this protocol analyzes"

phases:
  - id: "PHASE-01"
    name: "Phase Name"
    instructions: |
      Detailed instructions for the AI researcher...
    output_schema: "SchemaName"

  - id: "PHASE-02"
    name: "Analysis Phase"
    dependencies: ["PHASE-01"]
    instructions: |
      Build on previous phase results...

quality_gates:
  - phase: "PHASE-01"
    check: "Validation criteria"
    fail_action: "RETRY"
```

### Creating Custom Manifests

#### Using the Manifest Editor (Recommended)
1. Click the **"+ New"** button in the Manifests section of the sidebar
2. The Manifest Editor opens with a template
3. Edit the YAML content with your custom phases and instructions
4. Click **"Validate"** to check for errors
5. Click **"Save"** to save your manifest
6. Click **"Use Manifest"** to load it immediately

#### Editing Existing Manifests
1. Click the **"Edit"** (pencil icon) next to any manifest
2. Modify the YAML content in the editor
3. Validate and save your changes
4. Changes are immediately available for use

#### Loading External Files
1. Click **"Open"** in the Manifest Editor toolbar
2. Select any `.yaml` or `.yml` file from your system
3. Validate the content
4. Save to add it to your manifest library

#### Manual File Creation
You can also create manifests manually:
1. Create a new `.yaml` file in the `manifests/` directory
2. Follow the manifest schema structure
3. Restart the app to see it in the dropdown

## Architecture

### Technology Stack

- **Frontend**: React 18 + TypeScript + Vite
- **Backend**: Rust (Tauri v2)
- **Database**: SQLite with rusqlite
- **AI Integration**: Multi-provider support with streaming
  - Anthropic Claude (Claude Opus 4, Sonnet 4.5, Haiku 3.5)
  - OpenAI GPT (GPT-4o, GPT-4 Turbo, GPT-3.5 Turbo)
  - Google Gemini (Gemini 1.5 Pro, Gemini 1.5 Flash)
  - DeepSeek (DeepSeek Chat, DeepSeek Coder)
- **Styling**: CSS with design tokens

### Project Structure

```
ted_skinner_project/
├── src/                    # React frontend
│   ├── App.tsx            # Main application component
│   ├── App.css            # Application styles
│   └── components/        # UI components
├── src-tauri/             # Rust backend
│   ├── src/
│   │   ├── main.rs       # Tauri commands & app setup
│   │   ├── agent.rs      # Research workflow engine
│   │   ├── auth.rs       # Authentication & database
│   │   ├── llm.rs        # LLM API integration
│   │   └── manifest.rs   # Manifest parsing
│   └── tauri.conf.json   # Tauri configuration
├── manifests/             # Research protocol definitions
│   ├── fullintel_process_manifest.yaml
│   ├── Deep-Dive-Industry-Protocol.yaml
│   ├── frontier-tech-protocol.yaml
│   ├── Inductive-Company-Protocol.yaml
│   ├── Intellectual-capital-protocol.yaml
│   ├── value-chain-protocol.yaml
│   └── venture-momentum-protocol.yaml
└── docs/                  # Documentation
```

## Development

### Prerequisites

- [Node.js](https://nodejs.org/) 18+
- [Rust](https://rustup.rs/) 1.70+
- [Tauri CLI](https://tauri.app/)

#### Platform-Specific Dependencies

**Windows:**
- WebView2 (usually pre-installed on Windows 10/11)
- Visual Studio Build Tools with C++ workload

**macOS:**
- Xcode Command Line Tools: `xcode-select --install`

**Linux (Debian/Ubuntu):**
```bash
sudo apt install libgtk-3-dev libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev
```

### Setup

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production (current platform)
npm run tauri build

# Build for specific target
npm run tauri build -- --target x86_64-apple-darwin    # macOS Intel
npm run tauri build -- --target aarch64-apple-darwin   # macOS Apple Silicon
npm run tauri build -- --target x86_64-unknown-linux-gnu  # Linux
npm run tauri build -- --target x86_64-pc-windows-msvc    # Windows
```

### Testing

```bash
# Run all Rust tests
cd src-tauri && cargo test

# Run specific test battery
cargo test battery1  # Unit tests
cargo test battery2  # Integration tests
cargo test battery3  # System tests
```

## Configuration

### API Key Setup

Get an API key from any supported provider:

| Provider | Console URL | Notes |
|----------|-------------|-------|
| **Anthropic** | [console.anthropic.com](https://console.anthropic.com) | Claude models |
| **OpenAI** | [platform.openai.com](https://platform.openai.com) | GPT models |
| **Google** | [aistudio.google.com](https://aistudio.google.com) | Gemini models |
| **DeepSeek** | [platform.deepseek.com](https://platform.deepseek.com) | Cost-effective alternative |

Then in the app:
1. Click Settings (gear icon)
2. Select your provider from the dropdown
3. Enter your API key
4. Click Save

### Model Selection

Choose models based on your needs:

**Anthropic Claude**
- **Claude Opus 4** - Highest quality, complex analysis
- **Claude Sonnet 4.5** - Recommended balance of speed/quality
- **Claude Haiku 3.5** - Fastest, lower cost

**OpenAI GPT**
- **GPT-4o** - Latest multimodal model
- **GPT-4 Turbo** - High capability, faster than GPT-4
- **GPT-3.5 Turbo** - Fast and economical
- **o1-preview** - Advanced reasoning model
- **o3-mini** - Efficient reasoning model

**Google Gemini**
- **Gemini 1.5 Pro** - Advanced reasoning
- **Gemini 1.5 Flash** - Fast responses

**DeepSeek**
- **DeepSeek Chat** - General purpose
- **DeepSeek Coder** - Code-focused analysis

### Custom Providers

Add your own AI providers for additional flexibility:

1. **Open Settings** - Click the gear icon
2. **Go to Custom Providers** - Select the "Custom Providers" tab
3. **Add Provider** - Enter:
   - **Name**: Display name for the provider
   - **Base URL**: API endpoint (e.g., `http://localhost:11434/v1`)
   - **Model ID**: Model identifier (e.g., `llama3.2`)
   - **API Key**: Optional, depending on provider
4. **Save** - Your custom provider appears in the model dropdown

**Common Use Cases:**
- Local Ollama instances
- OpenAI-compatible APIs (Together AI, Groq, etc.)
- Self-hosted models
- Enterprise private deployments

## Data Storage

All user data is stored locally on your machine:

| Platform | Data Directory |
|----------|---------------|
| **Windows** | `%APPDATA%\com.fullintel.agent\` |
| **macOS** | `~/Library/Application Support/com.fullintel.agent/` |
| **Linux** | `~/.local/share/com.fullintel.agent/` |

| File | Purpose |
|------|---------|
| `users.db` | User database and research sessions |
| `config.json` | Application configuration |

No data is sent to external servers except for the AI API calls to your selected provider.

## Usage Guide

### Chat Mode

The application defaults to Chat Mode for quick AI conversations:

1. **Start Chatting** - Type your message in the input field and press Enter
2. **View Response** - AI responses stream in real-time
3. **Continue Conversation** - Ask follow-up questions to build context
4. **Switch Models** - Change AI models mid-conversation if needed

### Research Mode

For structured multi-phase research:

1. **Switch to Research** - Click the "Research" toggle button
2. **Select Manifest** - Choose a research protocol from the dropdown
3. **Enter Subject** - Type your research topic (company name, technology, etc.)
4. **Run Workflow** - Click "Run" to start the multi-phase research
5. **Monitor Progress** - Watch each phase complete with streaming output
6. **Follow Up** - Ask questions about the research after completion

### Session Management

#### Viewing Sessions
- **Session History** - All sessions listed in sidebar, organized by date
- **Session Details** - Click any session to view all phase outputs
- **Phase Expansion** - Click phase headers to expand/collapse outputs

#### Session Resume
- **Incomplete Sessions** - Sessions that didn't complete show "in_progress" status
- **Resume Button** - Click "Resume" to continue from the last completed phase
- **Context Preservation** - All previous phase outputs are used as context

#### Prompt Editing & Relaunch
- **View Prompts** - Click "View Prompts" on any phase to see system/user prompts
- **Edit Mode** - Click "Edit" to modify the prompts
- **Relaunch Phase** - Click "Relaunch" to re-run with original or modified prompts
- **Reproducibility** - Original prompts always preserved for reference

### Project Organization

Group related research sessions:

1. **Create Project** - Click "New Project" in the sidebar
2. **Name Project** - Give it a descriptive name
3. **Add Sessions** - Use session menu (⋯) → "Add to Project"
4. **View Project Sessions** - Click project to expand and see all sessions
5. **Archive Projects** - Archive completed projects to declutter

### Export Options

- **Copy** - Click copy icon to copy all output to clipboard
- **Export Markdown** - Use menu to save as `.md` file
- **Print** - Print-optimized view for physical copies
- **Per-Phase Export** - Copy individual phase outputs

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
| Send Message | `Enter` |
| New Line in Message | `Shift+Enter` |
| New Research | `Ctrl+N` |
| Settings | `Ctrl+,` |
| Export | `Ctrl+E` |

## Troubleshooting

### "API Key Invalid" Error
- Verify your API key at your provider's console:
  - Anthropic: [console.anthropic.com](https://console.anthropic.com)
  - OpenAI: [platform.openai.com](https://platform.openai.com)
  - Google: [aistudio.google.com](https://aistudio.google.com)
  - DeepSeek: [platform.deepseek.com](https://platform.deepseek.com)
- Ensure the key has not expired
- Check for extra spaces when pasting
- Verify you selected the correct provider in Settings

### App Won't Start

**Windows:**
- Ensure Windows 10/11 64-bit
- Try running as Administrator
- Check if WebView2 Runtime is installed

**macOS:**
- Right-click the app → Open (to bypass Gatekeeper)
- Check System Preferences → Security & Privacy if blocked
- Ensure macOS 10.15 or later

**Linux:**
- Check dependencies: `libgtk-3-0`, `libwebkit2gtk-4.1-0`
- For AppImage: ensure FUSE is installed (`sudo apt install fuse`)
- Run from terminal to see error messages

### Research Hangs or Fails
- Check your internet connection
- Verify API key is valid and has credits
- Check your provider's API status page
- Try a different model or provider

### Session Not Saving
- Ensure you're logged in
- Check disk space
- Restart the application

## Security

### Known Advisories

| Advisory | Severity | Platforms | Status |
|----------|----------|-----------|--------|
| [GHSA-wrw7-89jp-8q8g](https://github.com/advisories/GHSA-wrw7-89jp-8q8g) | Medium | Linux only | Upstream dependency |

**GHSA-wrw7-89jp-8q8g**: A memory safety issue in the `glib` crate (v0.18.5) affects Linux builds. This is a transitive dependency from Tauri's GTK/WebKit stack and cannot be patched until the upstream gtk-rs ecosystem releases glib 0.20.0 compatibility. Windows and macOS builds are unaffected. The vulnerability requires malformed GVariant data from an untrusted source to exploit, which is not a vector in this application's design.

### Reporting Security Issues

To report a security vulnerability, please open a private security advisory on GitHub or contact the maintainer directly.

## Support

For technical support, contact your administrator.

---

Built with [Tauri](https://tauri.app/) and [React](https://react.dev/)
