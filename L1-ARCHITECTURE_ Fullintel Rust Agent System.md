# **L1-ARCHITECTURE: Fullintel Rust Agent System**

Project: SE-CPM-FAST-AGENT  
Stack: Rust, Tauri v2, Tokio, Reqwest  
Target: Standalone Desktop App (Windows/macOS/Linux)

## **1\. System Overview**

The system acts as a "Manifest Execution Engine." Instead of hardcoding the Fullintel logic into Rust, the Rust system reads the fullintel\_process\_manifest.yaml and orchestrates AI agents to complete each step.

## **2\. Technical Stack (Fast & Standalone)**

| Component | Technology | Rationale |
| :---- | :---- | :---- |
| **Core Runtime** | **Rust (Tokio)** | Memory safety, concurrency for parallel research tasks, blazing speed. |
| **UI Framework** | **Tauri v2** | Uses native webview (tiny binary size), React/Typescript frontend, Rust backend. |
| **LLM Client** | **reqwest \+ serde** | Direct API calls to Claude/Gemini/DeepSeek. No bloated frameworks. |
| **State Mgmt** | **Sled** or **SQLite** | Embedded, file-based database for saving research sessions locally. |
| **Search Tool** | **Tavily API** or **Serper** | Essential for the "Research" phases (LLMs cannot do this alone).  |

## **3\. Component Architecture**

### **3.1 The "Agent Loop" (Rust Backend)**

This is the heart of the application. It runs in a background thread.

struct AgentState {  
    manifest: Manifest,  
    context: HashMap\<String, String\>, // Shared memory (The Blackboard)  
    history: Vec\<ChatMessage\>,  
}

impl Agent {  
    async fn execute\_phase(\&mut self, phase\_id: \&str) \-\> Result\<PhaseOutput\> {  
        // 1\. Load Phase instructions from Manifest  
        // 2\. Check dependencies (Wait if needed)  
        // 3\. Select Tool (Search vs LLM generation)  
        // 4\. Execute  
        // 5\. Validate Output against Schema  
        // 6\. Update Context  
    }  
}

### **3.2 The UI Layer (Tauri Frontend)**

The UI should not just be a chat box. It should be a **Live Dashboard**.

* **Left Panel:** The Manifest Checklist (Phase 1, Phase 2...). These light up green as the agent completes them.  
* **Center Panel:** The "Artifact Editor." As the agent generates the *Situation Summary* or *Outreach Email*, it streams directly into editable text blocks here.  
* **Right Panel:** "Thought Stream." Raw logs of the agent searching Google, scraping headers, and making decisions.

## **4\. Functional Requirements**

1. **Multi-Model Router:** The user must be able to select their preferred model (Claude 3.5 Sonnet for drafting, DeepSeek V3 for logic, Gemini Pro 1.5 for large context analysis) via a simple dropdown.  
2. **Tool Integration (Crucial):** The system **MUST** have a search capability. The Fullintel prompt requires finding *current* news (past 14 days).  
   * *Requirement:* Integration with **Tavily API** or **Google Custom Search API**.  
3. **Manifest Hot-Reloading:** The fullintel\_process\_manifest.yaml should be editable. If the sales process changes, you edit the YAML, not the Rust code.  
4. **Export Pipeline:** Button to export the final "Opportunity Brief" to PDF or copy to clipboard formatted for email.

## **5\. Critical "Other" Requirements (User Query Item 5\)**

To make this "Fast Moving" and "Production Ready," you strictly need:

1. **Rate Limiting & Cost Tracking:**  
   * *Why:* Parallel API calls to Anthropic/Google can drain wallets/limits fast.  
   * *Req:* A visible "Cost Meter" in the UI showing spend per report.  
2. **Headless Browser (Scraping):**  
   * *Why:* Identifying "Communications Team" (Step 3\) requires looking at LinkedIn or Team pages. Search APIs (snippets) are often insufficient.  
   * *Req:* Integration with a lightweight scraper (like headless\_chrome crate) to fetch full text from 2-3 key URLs.  
3. **Persistence:**  
   * *Why:* Research takes time. If the app closes, data is lost.  
   * *Req:* Auto-save every phase completion to local SQLite research.db.  
4. **API Key Vault:**  
   * *Req:* Secure local storage (OS Keychain via Tauri plugin) for API keys. Do not store in plaintext files.

## **6\. Project Directory Structure**

/fullintel-agent/  
├── src-tauri/           \# Rust Backend  
│   ├── src/  
│   │   ├── main.rs      \# Entry point  
│   │   ├── agent.rs     \# The execution loop  
│   │   ├── llm.rs       \# API clients (Claude/Gemini/DeepSeek)  
│   │   └── manifest.rs  \# YAML parser  
│   └── Cargo.toml  
├── src/                 \# Frontend (React/Vite)  
│   ├── components/  
│   │   ├── Dashboard.tsx  
│   │   └── Settings.tsx  
│   └── App.tsx  
├── manifests/  
│   └── fullintel\_v1.yaml  
└── requirements.md  
