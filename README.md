# FullIntel Agent

A desktop application for automated AI-powered research workflows. Execute multi-phase research protocols to generate comprehensive executive briefs on companies, industries, and emerging technologies.

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![Platform](https://img.shields.io/badge/platform-Windows%2010%2F11-lightgrey)

## Features

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
- [Anthropic API Key](https://console.anthropic.com) - Required for AI functionality

## Quick Start

1. **Create Account** - Set up a local user account on first launch
2. **Configure API Key** - Go to Settings (gear icon) and enter your Anthropic API key
3. **Select Manifest** - Choose a research protocol from the dropdown
4. **Enter Topic** - Type a company name or research subject
5. **Run Research** - Click "Run" and watch the results stream in

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
- **AI Integration**: Anthropic Claude API (streaming)
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

1. Visit [console.anthropic.com](https://console.anthropic.com)
2. Create an account and generate an API key
3. In the app, click Settings (gear icon)
4. Enter your API key and click Save

### Model Selection

Select from available Claude models in the dropdown:
- **Claude Sonnet 4.5** (Recommended - balanced speed/quality)
- **Claude Opus 4** (Highest quality, slower)
- **Claude Haiku 3.5** (Fastest, lower cost)

## Data Storage

All user data is stored locally on your machine:

| Data | Location |
|------|----------|
| User Database | `%APPDATA%\com.fullintel.agent\users.db` |
| Configuration | `%APPDATA%\com.fullintel.agent\config.json` |
| Research Sessions | Stored in SQLite database |

No data is sent to external servers except for the AI API calls to Anthropic.

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
- Verify your API key at [console.anthropic.com](https://console.anthropic.com)
- Ensure the key has not expired
- Check for extra spaces when pasting

### App Won't Start
- Ensure Windows 10/11 64-bit
- Try running as Administrator
- Check if WebView2 Runtime is installed

### Research Hangs or Fails
- Check your internet connection
- Verify API key is valid and has credits
- Check Anthropic API status at [status.anthropic.com](https://status.anthropic.com)
- Try a different model (Haiku is most reliable)

### Session Not Saving
- Ensure you're logged in
- Check disk space
- Restart the application

## Support

For technical support, contact your administrator.

---

Built with [Tauri](https://tauri.app/) and [React](https://react.dev/)
