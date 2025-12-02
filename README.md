# FullIntel Agent

A desktop application for automated AI-powered research workflows. Execute multi-phase research protocols to generate comprehensive executive briefs on companies, industries, and emerging technologies.

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey)

## Features

- **Multi-Provider AI Support** - Choose from Anthropic Claude, OpenAI GPT, Google Gemini, or DeepSeek
- **Multi-Phase Research Workflows** - Execute structured research protocols with AI-powered analysis
- **Real-Time Streaming** - Watch research results stream in as each phase completes
- **Multiple Research Manifests** - Choose from 7 pre-configured research protocols
- **Session Persistence** - Save and resume research sessions
- **Project Organization** - Group related research sessions into projects
- **Export Capabilities** - Export research results as Markdown
- **Secure Authentication** - Local user accounts with encrypted password storage

## Installation

### Windows Installer

1. Download `fullintel-agent_0.1.0_x64-setup.exe` from [Releases](../../releases)
2. Run the installer and follow the wizard
3. Launch "FullIntel Agent" from the Start Menu

### Requirements

- Windows 10/11 (64-bit)
- API key from at least one supported provider:
  - [Anthropic Claude](https://console.anthropic.com)
  - [OpenAI GPT](https://platform.openai.com)
  - [Google Gemini](https://aistudio.google.com)
  - [DeepSeek](https://platform.deepseek.com)

## Quick Start

1. **Create Account** - Set up a local user account on first launch
2. **Configure API Key** - Go to Settings (gear icon) and enter your API key for any supported provider
3. **Select Provider & Model** - Choose your AI provider and model from the dropdowns
4. **Select Manifest** - Choose a research protocol from the dropdown
5. **Enter Topic** - Type a company name or research subject
6. **Run Research** - Click "Run" and watch the results stream in

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

You can create your own research protocols by:
1. Creating a new `.yaml` file in the `manifests/` directory
2. Following the manifest schema structure
3. Defining phases with clear instructions
4. Adding quality gates for validation

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

### Setup

```bash
# Install dependencies
npm install

# Run in development mode
npm run tauri dev

# Build for production
npm run tauri build
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

**Google Gemini**
- **Gemini 1.5 Pro** - Advanced reasoning
- **Gemini 1.5 Flash** - Fast responses

**DeepSeek**
- **DeepSeek Chat** - General purpose
- **DeepSeek Coder** - Code-focused analysis

## Data Storage

All user data is stored locally on your machine:

| Data | Location |
|------|----------|
| User Database | `%APPDATA%\com.fullintel.agent\users.db` |
| Configuration | `%APPDATA%\com.fullintel.agent\config.json` |
| Research Sessions | Stored in SQLite database |

No data is sent to external servers except for the AI API calls to your selected provider.

## Usage Guide

### Running Research

1. Select a manifest from the dropdown (e.g., "Fullintel Process")
2. Enter your research subject in the input field
3. Click "Run" to start the workflow
4. Watch as each phase executes and streams results
5. View the complete brief when all phases finish

### Managing Sessions

- **View History** - Click sessions in the left sidebar
- **Resume Session** - Click on any historical session to view
- **Create Project** - Group related sessions into projects
- **Export Results** - Copy or export the research output

### Keyboard Shortcuts

| Action | Shortcut |
|--------|----------|
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
- Ensure Windows 10/11 64-bit
- Try running as Administrator
- Check if WebView2 Runtime is installed

### Research Hangs or Fails
- Check your internet connection
- Verify API key is valid and has credits
- Check your provider's API status page
- Try a different model or provider

### Session Not Saving
- Ensure you're logged in
- Check disk space
- Restart the application

## Support

For technical support, contact your administrator.

---

Built with [Tauri](https://tauri.app/) and [React](https://react.dev/)
