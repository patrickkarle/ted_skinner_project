import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event"; // Import event listener
import "./App.css";

type Phase = {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed";
};

// Define Event Payload Types
type LogPayload = { message: string };
type PhasePayload = { phase_id: string; status: string };

function App() {
  const [company, setCompany] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [logs, setLogs] = useState<string[]>([]);
  const [report, setReport] = useState("");
  const [isRunning, setIsRunning] = useState(false);

  // Initialize phases from Manifest ID structure (matching YAML)
  const [phases, setPhases] = useState<Phase[]>([
    { id: "PHASE-01-CONTEXT", name: "Context & Firmographics", status: "pending" },
    { id: "PHASE-02-SITUATION", name: "Situation Analysis", status: "pending" },
    { id: "PHASE-03-PAIN-MAPPING", name: "Pain Mapping", status: "pending" },
    { id: "PHASE-04-SOLUTION-MATCH", name: "Solution Matching", status: "pending" },
    { id: "PHASE-05-DRAFTING", name: "Drafting Brief", status: "pending" },
  ]);

  // Setup Event Listeners on Mount
  useEffect(() => {
    const unlistenLogs = listen<LogPayload>("agent-log", (event) => {
      setLogs((prev) => [...prev, `[AGENT] ${event.payload.message}`]);
    });

    const unlistenPhases = listen<PhasePayload>("phase-update", (event) => {
      setPhases((prev) => 
        prev.map((p) => 
          p.id === event.payload.phase_id 
            ? { ...p, status: event.payload.status as any } 
            : p
        )
      );
    });

    // Cleanup listeners on unmount
    return () => {
      unlistenLogs.then((f) => f());
      unlistenPhases.then((f) => f());
    };
  }, []);

  async function startResearch() {
    if (!apiKey) {
      alert("Please enter an API Key first.");
      return;
    }

    setIsRunning(true);
    setLogs([]); // Clear old logs
    setReport(""); // Clear old report
    // Reset phases
    setPhases(prev => prev.map(p => ({...p, status: "pending"})));

    try {
      await invoke("set_api_key", { key: apiKey });
      const result = await invoke<string>("run_research", { company });
      setReport(result);
    } catch (error) {
      console.error(error);
      setLogs((prev) => [...prev, `Error: ${error}`]);
    } finally {
      setIsRunning(false);
    }
  }

  return (
    <div className="container">
      <div className="sidebar">
        <h1>Fullintel Agent</h1>
        
        <div className="input-group">
          <label>API Key</label>
          <input 
            type="password" 
            value={apiKey} 
            onChange={(e) => setApiKey(e.target.value)} 
            placeholder="sk-..."
          />
        </div>

        <div className="input-group">
          <label>Target Company</label>
          <input 
            type="text" 
            value={company} 
            onChange={(e) => setCompany(e.target.value)} 
            placeholder="e.g. Tesla"
          />
        </div>

        <button onClick={startResearch} disabled={isRunning}>
          {isRunning ? "Agent Running..." : "Generate Brief"}
        </button>

        <div className="phase-list">
          <h3>Execution Plan</h3>
          {phases.map((phase) => (
            <div key={phase.id} className={`phase-item ${phase.status}`}>
              <span className={`status-dot ${phase.status}`}></span>
              {phase.name}
            </div>
          ))}
        </div>
      </div>

      <div className="main-content">
        <div className="tabs">
          <button className="active">Opportunity Brief</button>
        </div>

        {report ? (
          <div className="report-preview">
            <pre>{report}</pre>
          </div>
        ) : (
          <div className="empty-state">
            <p>Ready to research.</p>
            <div className="logs-preview">
              {logs.map((log, i) => <div key={i}>{log}</div>)}
            </div>
          </div>
        )}
      </div>
    </div>
  );
}

export default App;