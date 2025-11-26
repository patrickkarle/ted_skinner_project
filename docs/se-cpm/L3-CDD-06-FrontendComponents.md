# L3-CDD-06: Frontend Components Design Document

**Document ID:** L3-CDD-UI-001
**Component Name:** React Frontend (TypeScript)
**Version:** 1.0
**Date:** 2025-11-19
**Parent:** L2-ICD-01-TauriIPC.md
**Traceability:** L1-SAD REQ-SYS-006 (User Interface), SR-004 (Progressive Disclosure)

---

## 1. Component Overview

### 1.1 Purpose
Provides user interface for configuring research parameters, monitoring workflow progress, and viewing/exporting generated sales briefs using React and TypeScript.

### 1.2 Responsibilities
- Collect user input (company name, API keys)
- Invoke Tauri IPC commands to backend
- Display real-time workflow progress
- Render markdown output with syntax highlighting
- Provide export functionality (PDF, clipboard, markdown)
- Manage UI state and error handling
- Persist user preferences (theme, API keys)

### 1.3 Integration Points
| Component | Interface | Direction |
|-----------|-----------|-----------|
| Tauri Backend | IPC commands (`run_research`, etc.) | → Invokes Rust functions |
| Tauri Events | Event listeners (`workflow_started`, etc.) | ← Receives progress updates |
| Browser Storage | localStorage | → Persists UI preferences |

---

## 2. File Structure

```
src/
├── components/
│   ├── SetupScreen.tsx           # Initial configuration (company name, API keys)
│   ├── ProgressScreen.tsx        # Real-time workflow progress
│   ├── ResultsViewer.tsx         # Display markdown output
│   ├── SessionHistory.tsx        # List past research sessions
│   ├── SettingsPanel.tsx         # Configure API keys, preferences
│   └── shared/
│       ├── Button.tsx            # Reusable button component
│       ├── Input.tsx             # Reusable input component
│       ├── Card.tsx              # Card container component
│       └── ProgressBar.tsx       # Progress indicator
├── hooks/
│   ├── useTauriInvoke.ts         # Hook for Tauri IPC invocations
│   ├── useTauriEvent.ts          # Hook for Tauri event listeners
│   └── useLocalStorage.ts        # Hook for localStorage persistence
├── types/
│   ├── tauri.ts                  # TypeScript types for Tauri IPC
│   └── workflow.ts               # Workflow state types
├── utils/
│   ├── markdown.ts               # Markdown parsing utilities
│   └── export.ts                 # Export helpers (PDF, clipboard)
├── App.tsx                       # Main application component
├── main.tsx                      # Application entry point
└── styles/
    ├── globals.css               # Global styles
    └── themes.css                # Light/dark theme
```

---

## 3. TypeScript Types

### 3.1 Tauri IPC Types

```typescript
// src/types/tauri.ts

export interface ResearchResult {
  success: boolean;
  markdown_output?: string;
  error?: string;
  session_id: string;
  duration_ms: number;
  cost_usd: number;
}

export interface SessionSummary {
  session_id: string;
  company: string;
  created_at: number; // Unix timestamp
  status: 'completed' | 'failed' | 'running';
  duration_ms?: number;
  cost_usd?: number;
}

export interface ApiKeyConfig {
  anthropic?: string;
  google?: string;
  deepseek?: string;
  tavily?: string;
  newsapi?: string;
}

// Event payloads
export interface WorkflowStartedEvent {
  session_id: string;
  company: string;
  timestamp: number;
}

export interface PhaseStartedEvent {
  session_id: string;
  phase_id: string;
  phase_name: string;
  phase_number: number;
  timestamp: number;
}

export interface PhaseProgressEvent {
  session_id: string;
  phase_id: string;
  message: string;
  progress_percent?: number;
  timestamp: number;
}

export interface PhaseCompletedEvent {
  session_id: string;
  phase_id: string;
  output_preview: string;
  duration_ms: number;
  timestamp: number;
}

export interface WorkflowCompletedEvent {
  session_id: string;
  success: boolean;
  duration_ms: number;
  cost_usd: number;
  timestamp: number;
}

export interface WorkflowErrorEvent {
  session_id: string;
  phase_id: string;
  error_type: string;
  error_message: string;
  retry_count: number;
  timestamp: number;
}
```

### 3.2 Workflow State Types

```typescript
// src/types/workflow.ts

export type WorkflowStatus =
  | 'idle'
  | 'running'
  | 'completed'
  | 'failed'
  | 'cancelled';

export interface WorkflowState {
  status: WorkflowStatus;
  sessionId: string | null;
  company: string | null;
  currentPhase: number | null;
  totalPhases: number;
  progressMessage: string | null;
  error: string | null;
  result: ResearchResult | null;
}

export interface PhaseInfo {
  id: string;
  name: string;
  number: number;
  status: 'pending' | 'running' | 'completed' | 'failed';
  startedAt?: number;
  completedAt?: number;
  duration?: number;
  message?: string;
}
```

---

## 4. Custom Hooks

### 4.1 useTauriInvoke Hook

```typescript
// src/hooks/useTauriInvoke.ts
import { useState, useCallback } from 'react';
import { invoke } from '@tauri-apps/api/core';

interface InvokeState<T> {
  data: T | null;
  error: string | null;
  loading: boolean;
}

export function useTauriInvoke<T = unknown>(command: string) {
  const [state, setState] = useState<InvokeState<T>>({
    data: null,
    error: null,
    loading: false,
  });

  const execute = useCallback(
    async (args?: Record<string, unknown>) => {
      setState({ data: null, error: null, loading: true });

      try {
        const result = await invoke<T>(command, args);
        setState({ data: result, error: null, loading: false });
        return result;
      } catch (err) {
        const errorMessage = err instanceof Error ? err.message : String(err);
        setState({ data: null, error: errorMessage, loading: false });
        throw err;
      }
    },
    [command]
  );

  return { ...state, execute };
}
```

### 4.2 useTauriEvent Hook

```typescript
// src/hooks/useTauriEvent.ts
import { useEffect } from 'react';
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export function useTauriEvent<T>(
  eventName: string,
  handler: (payload: T) => void
) {
  useEffect(() => {
    let unlisten: UnlistenFn | null = null;

    const setupListener = async () => {
      unlisten = await listen<T>(eventName, (event) => {
        handler(event.payload);
      });
    };

    setupListener();

    return () => {
      if (unlisten) {
        unlisten();
      }
    };
  }, [eventName, handler]);
}
```

### 4.3 useLocalStorage Hook

```typescript
// src/hooks/useLocalStorage.ts
import { useState, useEffect } from 'react';

export function useLocalStorage<T>(
  key: string,
  initialValue: T
): [T, (value: T) => void] {
  const [storedValue, setStoredValue] = useState<T>(() => {
    try {
      const item = window.localStorage.getItem(key);
      return item ? JSON.parse(item) : initialValue;
    } catch (error) {
      console.error(`Error reading localStorage key "${key}":`, error);
      return initialValue;
    }
  });

  const setValue = (value: T) => {
    try {
      setStoredValue(value);
      window.localStorage.setItem(key, JSON.stringify(value));
    } catch (error) {
      console.error(`Error writing localStorage key "${key}":`, error);
    }
  };

  return [storedValue, setValue];
}
```

---

## 5. Component Implementations

### 5.1 SetupScreen Component

```typescript
// src/components/SetupScreen.tsx
import React, { useState } from 'react';
import { useTauriInvoke } from '../hooks/useTauriInvoke';
import { ResearchResult } from '../types/tauri';
import Button from './shared/Button';
import Input from './shared/Input';
import Card from './shared/Card';

interface SetupScreenProps {
  onResearchStarted: (sessionId: string, company: string) => void;
}

export default function SetupScreen({ onResearchStarted }: SetupScreenProps) {
  const [company, setCompany] = useState('');
  const { execute: runResearch, loading, error } = useTauriInvoke<ResearchResult>('run_research');

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!company.trim()) {
      alert('Please enter a company name');
      return;
    }

    try {
      const result = await runResearch({ company: company.trim() });
      onResearchStarted(result.session_id, company.trim());
    } catch (err) {
      console.error('Research failed:', err);
    }
  };

  return (
    <div className="setup-screen">
      <Card>
        <h1>Fullintel Sales Intelligence Generator</h1>
        <p className="subtitle">
          Generate comprehensive sales briefs in under 5 minutes
        </p>

        <form onSubmit={handleSubmit} className="setup-form">
          <Input
            label="Target Company"
            placeholder="e.g., Microsoft, Salesforce, Adobe"
            value={company}
            onChange={(e) => setCompany(e.target.value)}
            maxLength={200}
            disabled={loading}
            autoFocus
          />

          <div className="form-footer">
            <Button
              type="submit"
              variant="primary"
              size="large"
              loading={loading}
              disabled={!company.trim() || loading}
            >
              {loading ? 'Starting Research...' : 'Generate Brief'}
            </Button>
          </div>

          {error && (
            <div className="error-message">
              <strong>Error:</strong> {error}
            </div>
          )}
        </form>

        <div className="info-box">
          <h3>What happens next?</h3>
          <ul>
            <li>5-phase automated research workflow</li>
            <li>Real-time progress updates</li>
            <li>Quality validation at each step</li>
            <li>Cost: ~$0.05-0.10 per brief</li>
            <li>Time: 3-5 minutes average</li>
          </ul>
        </div>
      </Card>
    </div>
  );
}
```

### 5.2 ProgressScreen Component

```typescript
// src/components/ProgressScreen.tsx
import React, { useState, useCallback } from 'react';
import { useTauriEvent } from '../hooks/useTauriEvent';
import {
  PhaseStartedEvent,
  PhaseProgressEvent,
  PhaseCompletedEvent,
  WorkflowCompletedEvent,
  WorkflowErrorEvent,
} from '../types/tauri';
import { PhaseInfo } from '../types/workflow';
import Card from './shared/Card';
import ProgressBar from './shared/ProgressBar';

interface ProgressScreenProps {
  sessionId: string;
  company: string;
  onCompleted: (sessionId: string) => void;
  onError: (error: string) => void;
}

export default function ProgressScreen({
  sessionId,
  company,
  onCompleted,
  onError,
}: ProgressScreenProps) {
  const [phases, setPhases] = useState<PhaseInfo[]>([
    { id: 'phase_1', name: 'Company Context Research', number: 1, status: 'pending' },
    { id: 'phase_2', name: 'Situation Analysis', number: 2, status: 'pending' },
    { id: 'phase_3', name: 'Communications Intelligence', number: 3, status: 'pending' },
    { id: 'phase_4', name: 'Solution Matching', number: 4, status: 'pending' },
    { id: 'phase_5', name: 'Brief Generation', number: 5, status: 'pending' },
  ]);
  const [currentMessage, setCurrentMessage] = useState('Initializing workflow...');
  const [totalCost, setTotalCost] = useState(0);
  const [elapsedTime, setElapsedTime] = useState(0);

  // Phase started
  useTauriEvent<PhaseStartedEvent>('phase_started', useCallback((payload) => {
    if (payload.session_id !== sessionId) return;

    setPhases((prev) =>
      prev.map((phase) =>
        phase.id === payload.phase_id
          ? { ...phase, status: 'running', startedAt: payload.timestamp }
          : phase
      )
    );
    setCurrentMessage(`Starting: ${payload.phase_name}`);
  }, [sessionId]));

  // Phase progress
  useTauriEvent<PhaseProgressEvent>('phase_progress', useCallback((payload) => {
    if (payload.session_id !== sessionId) return;
    setCurrentMessage(payload.message);
  }, [sessionId]));

  // Phase completed
  useTauriEvent<PhaseCompletedEvent>('phase_completed', useCallback((payload) => {
    if (payload.session_id !== sessionId) return;

    setPhases((prev) =>
      prev.map((phase) =>
        phase.id === payload.phase_id
          ? {
              ...phase,
              status: 'completed',
              completedAt: payload.timestamp,
              duration: payload.duration_ms,
            }
          : phase
      )
    );
    setCurrentMessage(`Completed: ${payload.phase_id}`);
  }, [sessionId]));

  // Workflow completed
  useTauriEvent<WorkflowCompletedEvent>('workflow_completed', useCallback((payload) => {
    if (payload.session_id !== sessionId) return;

    setTotalCost(payload.cost_usd);
    setElapsedTime(payload.duration_ms);

    if (payload.success) {
      onCompleted(sessionId);
    }
  }, [sessionId, onCompleted]));

  // Workflow error
  useTauriEvent<WorkflowErrorEvent>('workflow_error', useCallback((payload) => {
    if (payload.session_id !== sessionId) return;

    setPhases((prev) =>
      prev.map((phase) =>
        phase.id === payload.phase_id
          ? { ...phase, status: 'failed' }
          : phase
      )
    );
    onError(payload.error_message);
  }, [sessionId, onError]));

  const completedPhases = phases.filter((p) => p.status === 'completed').length;
  const progressPercent = (completedPhases / phases.length) * 100;

  return (
    <div className="progress-screen">
      <Card>
        <h1>Generating Brief for {company}</h1>
        <p className="session-id">Session: {sessionId.slice(0, 8)}...</p>

        <ProgressBar value={progressPercent} max={100} />

        <div className="phase-list">
          {phases.map((phase) => (
            <div key={phase.id} className={`phase-item phase-${phase.status}`}>
              <div className="phase-icon">
                {phase.status === 'completed' && '✓'}
                {phase.status === 'running' && '⟳'}
                {phase.status === 'pending' && '○'}
                {phase.status === 'failed' && '✗'}
              </div>
              <div className="phase-info">
                <div className="phase-name">{phase.name}</div>
                {phase.duration && (
                  <div className="phase-duration">
                    {(phase.duration / 1000).toFixed(1)}s
                  </div>
                )}
              </div>
            </div>
          ))}
        </div>

        <div className="status-message">{currentMessage}</div>

        <div className="metrics">
          <div className="metric">
            <span className="metric-label">Elapsed:</span>
            <span className="metric-value">{(elapsedTime / 1000).toFixed(1)}s</span>
          </div>
          <div className="metric">
            <span className="metric-label">Cost:</span>
            <span className="metric-value">${totalCost.toFixed(4)}</span>
          </div>
        </div>
      </Card>
    </div>
  );
}
```

### 5.3 ResultsViewer Component

```typescript
// src/components/ResultsViewer.tsx
import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import ReactMarkdown from 'react-markdown';
import { Prism as SyntaxHighlighter } from 'react-syntax-highlighter';
import { tomorrow } from 'react-syntax-highlighter/dist/esm/styles/prism';
import Button from './shared/Button';
import Card from './shared/Card';

interface ResultsViewerProps {
  sessionId: string;
  onBack: () => void;
}

export default function ResultsViewer({ sessionId, onBack }: ResultsViewerProps) {
  const [markdown, setMarkdown] = useState('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const loadOutput = async () => {
      try {
        setLoading(true);
        const output = await invoke<string>('get_session_output', { sessionId });
        setMarkdown(output);
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : String(err));
      } finally {
        setLoading(false);
      }
    };

    loadOutput();
  }, [sessionId]);

  const handleCopyToClipboard = async () => {
    try {
      await invoke('copy_to_clipboard', { sessionId });
      alert('Brief copied to clipboard!');
    } catch (err) {
      alert('Failed to copy: ' + err);
    }
  };

  const handleExportPdf = async () => {
    try {
      const path = await invoke<string>('export_to_pdf', {
        sessionId,
        outputPath: null, // Use default Downloads folder
      });
      alert(`PDF exported to: ${path}`);
    } catch (err) {
      alert('Failed to export PDF: ' + err);
    }
  };

  if (loading) {
    return (
      <div className="results-viewer">
        <Card>
          <div className="loading-state">Loading brief...</div>
        </Card>
      </div>
    );
  }

  if (error) {
    return (
      <div className="results-viewer">
        <Card>
          <div className="error-state">
            <strong>Error:</strong> {error}
          </div>
          <Button onClick={onBack}>Back to Setup</Button>
        </Card>
      </div>
    );
  }

  return (
    <div className="results-viewer">
      <Card>
        <div className="results-header">
          <h1>Sales Intelligence Brief</h1>
          <div className="results-actions">
            <Button variant="secondary" onClick={handleCopyToClipboard}>
              📋 Copy
            </Button>
            <Button variant="secondary" onClick={handleExportPdf}>
              📄 Export PDF
            </Button>
            <Button variant="primary" onClick={onBack}>
              ← New Research
            </Button>
          </div>
        </div>

        <div className="markdown-content">
          <ReactMarkdown
            components={{
              code({ node, inline, className, children, ...props }) {
                const match = /language-(\w+)/.exec(className || '');
                return !inline && match ? (
                  <SyntaxHighlighter
                    style={tomorrow}
                    language={match[1]}
                    PreTag="div"
                    {...props}
                  >
                    {String(children).replace(/\n$/, '')}
                  </SyntaxHighlighter>
                ) : (
                  <code className={className} {...props}>
                    {children}
                  </code>
                );
              },
            }}
          >
            {markdown}
          </ReactMarkdown>
        </div>
      </Card>
    </div>
  );
}
```

### 5.4 SessionHistory Component

```typescript
// src/components/SessionHistory.tsx
import React, { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { SessionSummary } from '../types/tauri';
import Button from './shared/Button';
import Card from './shared/Card';

interface SessionHistoryProps {
  onViewSession: (sessionId: string) => void;
}

export default function SessionHistory({ onViewSession }: SessionHistoryProps) {
  const [sessions, setSessions] = useState<SessionSummary[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const loadHistory = async () => {
      try {
        const history = await invoke<SessionSummary[]>('get_session_history', {
          limit: 50,
        });
        setSessions(history);
      } catch (err) {
        console.error('Failed to load history:', err);
      } finally {
        setLoading(false);
      }
    };

    loadHistory();
  }, []);

  if (loading) {
    return <div className="loading-state">Loading history...</div>;
  }

  return (
    <Card>
      <h2>Research History</h2>

      {sessions.length === 0 ? (
        <div className="empty-state">No research sessions yet.</div>
      ) : (
        <div className="session-list">
          {sessions.map((session) => (
            <div key={session.session_id} className="session-item">
              <div className="session-info">
                <div className="session-company">{session.company}</div>
                <div className="session-meta">
                  {new Date(session.created_at).toLocaleString()} •{' '}
                  {session.status} •{' '}
                  {session.duration_ms
                    ? `${(session.duration_ms / 1000).toFixed(1)}s`
                    : 'N/A'}{' '}
                  • ${session.cost_usd?.toFixed(4) || '0.0000'}
                </div>
              </div>
              {session.status === 'completed' && (
                <Button
                  variant="secondary"
                  size="small"
                  onClick={() => onViewSession(session.session_id)}
                >
                  View
                </Button>
              )}
            </div>
          ))}
        </div>
      )}
    </Card>
  );
}
```

### 5.5 SettingsPanel Component

```typescript
// src/components/SettingsPanel.tsx
import React, { useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { useLocalStorage } from '../hooks/useLocalStorage';
import { ApiKeyConfig } from '../types/tauri';
import Button from './shared/Button';
import Input from './shared/Input';
import Card from './shared/Card';

export default function SettingsPanel() {
  const [apiKeys, setApiKeys] = useLocalStorage<ApiKeyConfig>('api_keys', {});
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleSave = async () => {
    try {
      setSaving(true);
      await invoke('save_api_keys', { keys: apiKeys });
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (err) {
      alert('Failed to save API keys: ' + err);
    } finally {
      setSaving(false);
    }
  };

  return (
    <Card>
      <h2>API Configuration</h2>
      <p className="subtitle">
        Your API keys are encrypted and stored securely on your device.
      </p>

      <div className="settings-form">
        <Input
          label="Anthropic API Key"
          type="password"
          placeholder="sk-ant-..."
          value={apiKeys.anthropic || ''}
          onChange={(e) =>
            setApiKeys({ ...apiKeys, anthropic: e.target.value })
          }
        />

        <Input
          label="Google API Key"
          type="password"
          placeholder="AIza..."
          value={apiKeys.google || ''}
          onChange={(e) =>
            setApiKeys({ ...apiKeys, google: e.target.value })
          }
        />

        <Input
          label="DeepSeek API Key"
          type="password"
          placeholder="sk-..."
          value={apiKeys.deepseek || ''}
          onChange={(e) =>
            setApiKeys({ ...apiKeys, deepseek: e.target.value })
          }
        />

        <Input
          label="Tavily API Key (optional)"
          type="password"
          placeholder="tvly-..."
          value={apiKeys.tavily || ''}
          onChange={(e) =>
            setApiKeys({ ...apiKeys, tavily: e.target.value })
          }
        />

        <Input
          label="NewsAPI Key (optional)"
          type="password"
          placeholder="..."
          value={apiKeys.newsapi || ''}
          onChange={(e) =>
            setApiKeys({ ...apiKeys, newsapi: e.target.value })
          }
        />

        <div className="form-footer">
          <Button
            variant="primary"
            onClick={handleSave}
            loading={saving}
            disabled={saving}
          >
            {saving ? 'Saving...' : saved ? 'Saved ✓' : 'Save API Keys'}
          </Button>
        </div>
      </div>
    </Card>
  );
}
```

---

## 6. Shared Components

### 6.1 Button Component

```typescript
// src/components/shared/Button.tsx
import React from 'react';

interface ButtonProps extends React.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: 'primary' | 'secondary' | 'danger';
  size?: 'small' | 'medium' | 'large';
  loading?: boolean;
}

export default function Button({
  variant = 'primary',
  size = 'medium',
  loading = false,
  disabled,
  children,
  className = '',
  ...props
}: ButtonProps) {
  return (
    <button
      className={`btn btn-${variant} btn-${size} ${className}`}
      disabled={disabled || loading}
      {...props}
    >
      {loading ? <span className="spinner" /> : children}
    </button>
  );
}
```

### 6.2 Input Component

```typescript
// src/components/shared/Input.tsx
import React from 'react';

interface InputProps extends React.InputHTMLAttributes<HTMLInputElement> {
  label?: string;
  error?: string;
}

export default function Input({
  label,
  error,
  className = '',
  ...props
}: InputProps) {
  return (
    <div className="input-group">
      {label && <label className="input-label">{label}</label>}
      <input className={`input ${error ? 'input-error' : ''} ${className}`} {...props} />
      {error && <div className="input-error-message">{error}</div>}
    </div>
  );
}
```

### 6.3 ProgressBar Component

```typescript
// src/components/shared/ProgressBar.tsx
import React from 'react';

interface ProgressBarProps {
  value: number;
  max: number;
}

export default function ProgressBar({ value, max }: ProgressBarProps) {
  const percentage = Math.min((value / max) * 100, 100);

  return (
    <div className="progress-bar-container">
      <div className="progress-bar-fill" style={{ width: `${percentage}%` }} />
      <div className="progress-bar-label">{Math.round(percentage)}%</div>
    </div>
  );
}
```

---

## 7. Main Application Component

### 7.1 App.tsx

```typescript
// src/App.tsx
import React, { useState } from 'react';
import SetupScreen from './components/SetupScreen';
import ProgressScreen from './components/ProgressScreen';
import ResultsViewer from './components/ResultsViewer';
import SessionHistory from './components/SessionHistory';
import SettingsPanel from './components/SettingsPanel';
import { WorkflowStatus } from './types/workflow';

export default function App() {
  const [currentView, setCurrentView] = useState<
    'setup' | 'progress' | 'results' | 'history' | 'settings'
  >('setup');
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [company, setCompany] = useState<string | null>(null);

  const handleResearchStarted = (newSessionId: string, companyName: string) => {
    setSessionId(newSessionId);
    setCompany(companyName);
    setCurrentView('progress');
  };

  const handleWorkflowCompleted = (completedSessionId: string) => {
    setSessionId(completedSessionId);
    setCurrentView('results');
  };

  const handleWorkflowError = (error: string) => {
    alert(`Workflow failed: ${error}`);
    setCurrentView('setup');
  };

  const handleBackToSetup = () => {
    setSessionId(null);
    setCompany(null);
    setCurrentView('setup');
  };

  return (
    <div className="app">
      <nav className="app-nav">
        <button onClick={() => setCurrentView('setup')}>New Research</button>
        <button onClick={() => setCurrentView('history')}>History</button>
        <button onClick={() => setCurrentView('settings')}>Settings</button>
      </nav>

      <main className="app-main">
        {currentView === 'setup' && (
          <SetupScreen onResearchStarted={handleResearchStarted} />
        )}

        {currentView === 'progress' && sessionId && company && (
          <ProgressScreen
            sessionId={sessionId}
            company={company}
            onCompleted={handleWorkflowCompleted}
            onError={handleWorkflowError}
          />
        )}

        {currentView === 'results' && sessionId && (
          <ResultsViewer sessionId={sessionId} onBack={handleBackToSetup} />
        )}

        {currentView === 'history' && (
          <SessionHistory
            onViewSession={(id) => {
              setSessionId(id);
              setCurrentView('results');
            }}
          />
        )}

        {currentView === 'settings' && <SettingsPanel />}
      </main>
    </div>
  );
}
```

---

## 8. Dependencies (package.json)

```json
{
  "name": "fullintel-agent-ui",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview"
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "react": "^18.3.1",
    "react-dom": "^18.3.1",
    "react-markdown": "^9.0.1",
    "react-syntax-highlighter": "^15.5.0"
  },
  "devDependencies": {
    "@types/react": "^18.3.1",
    "@types/react-dom": "^18.3.0",
    "@types/react-syntax-highlighter": "^15.5.11",
    "@vitejs/plugin-react": "^4.2.1",
    "typescript": "^5.3.3",
    "vite": "^5.1.4"
  }
}
```

---

## 9. Styling Guidelines

### 9.1 CSS Variables (globals.css)

```css
/* src/styles/globals.css */
:root {
  /* Colors */
  --color-primary: #0066cc;
  --color-primary-hover: #0052a3;
  --color-secondary: #6c757d;
  --color-danger: #dc3545;
  --color-success: #28a745;
  --color-warning: #ffc107;

  /* Backgrounds */
  --bg-primary: #ffffff;
  --bg-secondary: #f8f9fa;
  --bg-tertiary: #e9ecef;

  /* Text */
  --text-primary: #212529;
  --text-secondary: #6c757d;
  --text-muted: #adb5bd;

  /* Borders */
  --border-color: #dee2e6;
  --border-radius: 8px;

  /* Spacing */
  --spacing-xs: 4px;
  --spacing-sm: 8px;
  --spacing-md: 16px;
  --spacing-lg: 24px;
  --spacing-xl: 32px;

  /* Shadows */
  --shadow-sm: 0 1px 3px rgba(0, 0, 0, 0.1);
  --shadow-md: 0 4px 6px rgba(0, 0, 0, 0.1);
  --shadow-lg: 0 10px 15px rgba(0, 0, 0, 0.1);
}

/* Dark theme */
[data-theme='dark'] {
  --bg-primary: #1a1a1a;
  --bg-secondary: #2d2d2d;
  --bg-tertiary: #3a3a3a;
  --text-primary: #f8f9fa;
  --text-secondary: #adb5bd;
  --border-color: #495057;
}
```

---

## 10. Error Handling

### 10.1 Error Boundary Component

```typescript
// src/components/ErrorBoundary.tsx
import React, { Component, ErrorInfo, ReactNode } from 'react';

interface Props {
  children: ReactNode;
}

interface State {
  hasError: boolean;
  error: Error | null;
}

export default class ErrorBoundary extends Component<Props, State> {
  constructor(props: Props) {
    super(props);
    this.state = { hasError: false, error: null };
  }

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error };
  }

  componentDidCatch(error: Error, errorInfo: ErrorInfo) {
    console.error('Uncaught error:', error, errorInfo);
  }

  render() {
    if (this.state.hasError) {
      return (
        <div className="error-boundary">
          <h1>Something went wrong</h1>
          <details>
            <summary>Error details</summary>
            <pre>{this.state.error?.toString()}</pre>
          </details>
          <button onClick={() => window.location.reload()}>
            Reload Application
          </button>
        </div>
      );
    }

    return this.props.children;
  }
}
```

---

## 11. Testing Requirements

### 11.1 Component Tests (Vitest)

```typescript
// src/components/__tests__/SetupScreen.test.tsx
import { describe, it, expect, vi } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import SetupScreen from '../SetupScreen';

describe('SetupScreen', () => {
  it('renders input and submit button', () => {
    render(<SetupScreen onResearchStarted={vi.fn()} />);

    expect(screen.getByPlaceholderText(/e.g., Microsoft/i)).toBeInTheDocument();
    expect(screen.getByText(/Generate Brief/i)).toBeInTheDocument();
  });

  it('validates empty input', async () => {
    const mockStart = vi.fn();
    render(<SetupScreen onResearchStarted={mockStart} />);

    const button = screen.getByText(/Generate Brief/i);
    fireEvent.click(button);

    expect(mockStart).not.toHaveBeenCalled();
  });

  it('submits valid company name', async () => {
    const mockStart = vi.fn();
    render(<SetupScreen onResearchStarted={mockStart} />);

    const input = screen.getByPlaceholderText(/e.g., Microsoft/i);
    const button = screen.getByText(/Generate Brief/i);

    fireEvent.change(input, { target: { value: 'TechCorp' } });
    fireEvent.click(button);

    // Mock Tauri invoke should trigger onResearchStarted
    // (Requires mocking @tauri-apps/api/core)
  });
});
```

---

## 12. Performance Requirements

| Metric | Target | Validation Method |
|--------|--------|------------------|
| **Initial Load** | < 2s | Lighthouse audit |
| **Component Render** | < 100ms | React DevTools Profiler |
| **Event Listener Setup** | < 50ms | Performance timing |
| **Markdown Rendering** | < 500ms | Time 5000-word document |
| **Memory Usage** | < 150MB | Chrome DevTools Memory |

---

## 13. Accessibility Requirements

### 13.1 WCAG 2.1 AA Compliance

```typescript
// Keyboard navigation
<Button
  aria-label="Generate sales intelligence brief"
  tabIndex={0}
  onKeyDown={(e) => {
    if (e.key === 'Enter' || e.key === ' ') {
      handleSubmit();
    }
  }}
>
  Generate Brief
</Button>

// Screen reader announcements
<div role="status" aria-live="polite" aria-atomic="true">
  {currentMessage}
</div>

// Focus management
useEffect(() => {
  if (currentView === 'results') {
    document.querySelector('.results-header h1')?.focus();
  }
}, [currentView]);
```

---

## 14. Traceability Matrix

| L2 Interface Requirement | Implementation Element | Validation |
|-------------------------|----------------------|------------|
| ICD-01: run_research command | SetupScreen.handleSubmit | Component test |
| ICD-01: Event listeners | useTauriEvent hook | Integration test |
| ICD-01: get_session_history | SessionHistory component | Component test |
| ICD-01: export_to_pdf | ResultsViewer.handleExportPdf | E2E test |
| L1-SAD SR-004 (Progressive disclosure) | 3-screen flow | User testing |

---

**Document Status:** Complete - Ready for L4-Manifest
**Next Phase:** PHASE 6: MANIFEST - Create L4 Implementation Inventory with taxonomy codes
