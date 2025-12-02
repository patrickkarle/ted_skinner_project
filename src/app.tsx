import React, { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { save, open } from "@tauri-apps/plugin-dialog";
import { writeTextFile } from "@tauri-apps/plugin-fs";
import "./App.css";
import { AuthScreen } from "./components/AuthScreen";
import { SettingsPanel } from "./components/SettingsPanel";
import { ManifestEditor } from "./components/ManifestEditor";
import fullintelLogoWide from "./assets/fullintel_logo_wide.jpg";

type Phase = {
  id: string;
  name: string;
  status: "pending" | "running" | "completed" | "failed";
};

type PhaseInfo = {
  id: string;
  name: string;
};

type SavedManifest = {
  name: string;
  path: string;
};

type UserInfo = {
  id: number;
  username: string;
  first_name?: string | null;
  last_name?: string | null;
  role?: string | null;
  location?: string | null;
};

// Brief types for persistence
type BriefSummary = {
  id: number;
  company: string;
  model: string;
  manifest_name: string | null;
  created_at: string;
};

type Brief = BriefSummary & {
  user_id: number;
  content: string;
};

type PersistedConversationMessage = {
  id: number;
  brief_id: number;
  role: string;
  content: string;
  created_at: string;
};

// Custom provider type (matches SettingsPanel)
type CustomProviderSummary = {
  id: number;
  name: string;
  provider_key: string;
  model_id: string;
  has_key: boolean;
};

type LogPayload = { message: string };
type PhasePayload = { phase_id: string; status: string };
type StreamPayload = { token: string; phase_id: string };

// Phase output payload for session persistence (matches Rust PhaseOutputPayload)
// IM-5001, IM-5002: Extended with system_prompt and user_input for user data accessibility
type PhaseOutputPayload = {
  session_id: number | null;
  phase_id: string;
  phase_name: string;
  status: string;  // "running", "completed", "failed"
  system_prompt: string | null;  // IM-5001: System prompt sent to LLM
  user_input: string | null;     // IM-5002: User input/manifest data sent to LLM
  output: string | null;
  error: string | null;
};

// Research session types (matches Rust structs)
type ResearchSessionSummary = {
  id: number;
  company: string;
  model: string;
  manifest_name: string | null;
  status: string;  // "in_progress", "completed", "failed"
  current_phase_id: string | null;
  archived: boolean;
  created_at: string;
  updated_at: string;
  project_id: number | null;
  project_name: string | null;
};

// IM-5001, IM-5002: Extended with system_prompt and user_input for user data accessibility
type PhaseOutputRecord = {
  id: number;
  session_id: number;
  phase_id: string;
  phase_name: string;
  status: string;
  system_prompt: string | null;  // IM-5001: System prompt sent to LLM
  user_input: string | null;     // IM-5002: User input/manifest data sent to LLM
  output: string | null;
  error: string | null;
  created_at: string;
  updated_at: string;
};

// IM-5030: Session-level conversation message (separate from brief-level)
// Reserved for future conversation tracking feature - uncomment when implementing
// type SessionMessage = {
//   id: number;
//   session_id: number;
//   phase_id: string | null;
//   role: string;  // "user", "assistant", "system"
//   content: string;
//   created_at: string;
// };

// IM-5020: Session resume types for reconstructing context
type SessionHistoryMessage = {
  role: string;  // "user" or "assistant"
  content: string;
};

type SessionContext = {
  history: SessionHistoryMessage[];
  last_completed_phase: string;
  total_phases: number;
  completed_phases: number;
};

// Full session data with complete context for resume
type ResearchSession = {
  id: number;
  user_id: number;
  company: string;
  model: string;
  manifest_path: string | null;
  manifest_name: string | null;
  status: string;
  current_phase_id: string | null;
  created_at: string;
  updated_at: string;
};

type ResumeSessionResult = {
  session: ResearchSession;
  next_phase_id: string;
  context: SessionContext;
};

// Project types for organizing sessions into groups
type Project = {
  id: number;
  user_id: number;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
};

type ProjectSummary = {
  id: number;
  name: string;
  description: string | null;
  session_count: number;
  archived: boolean;
  created_at: string;
  updated_at: string;
};

// Helper to get provider info for built-in providers
const getProviderInfo = (model: string): { name: string; provider: string } => {
  if (model.startsWith("claude")) return { name: "Anthropic", provider: "anthropic" };
  if (model.startsWith("deepseek")) return { name: "DeepSeek", provider: "deepseek" };
  if (model.startsWith("gemini")) return { name: "Google", provider: "google" };
  if (model.startsWith("gpt")) return { name: "OpenAI", provider: "openai" };
  return { name: "OpenAI", provider: "openai" };
};

// Helper to get provider info including custom providers
const getProviderInfoWithCustom = (
  model: string,
  customProviders: CustomProviderSummary[]
): { name: string; provider: string; isCustom: boolean } => {
  // Check if this is a custom provider
  if (model.startsWith("custom_")) {
    const customProvider = customProviders.find(p => p.provider_key === model);
    if (customProvider) {
      return { name: customProvider.name, provider: model, isCustom: true };
    }
    return { name: "Custom Provider", provider: model, isCustom: true };
  }
  // Built-in provider
  const info = getProviderInfo(model);
  return { ...info, isCustom: false };
};

type ConversationMessage = {
  role: "user" | "assistant";
  content: string;
};

// Default phases (used when no manifest is loaded)
const defaultPhases: Phase[] = [
  { id: "PHASE-01-CONTEXT", name: "Context & Firmographics", status: "pending" },
  { id: "PHASE-02-SITUATION", name: "Situation Analysis", status: "pending" },
  { id: "PHASE-03-PAIN-MAPPING", name: "Pain Mapping", status: "pending" },
  { id: "PHASE-04-SOLUTION-MATCH", name: "Solution Matching", status: "pending" },
  { id: "PHASE-05-DRAFTING", name: "Drafting Brief", status: "pending" },
];

// Helper to create short display names for phases (max 2 words)
const getPhaseDisplayName = (name: string): string => {
  // Common shortening patterns
  const shortenings: Record<string, string> = {
    "Context & Firmographics": "Context",
    "Situation Analysis": "Situation",
    "Pain Mapping": "Pain Mapping",
    "Solution Matching": "Solutions",
    "Drafting Brief": "Brief",
    "Macro Landscape Survey": "Landscape",
    "Structural Breakdown": "Breakdown",
    "Emergent Entity Identification": "Players",
    "Technological Deep Dive": "Tech Vectors",
    "Fidelity Check": "Verification",
    "Strategic Manifesto Generation": "Synthesis",
  };

  // Check for exact match first
  if (shortenings[name]) return shortenings[name];

  // Otherwise, take first two words (or just first if single word)
  const words = name.split(' ');
  if (words.length <= 2) return name;
  return words.slice(0, 2).join(' ');
};

function App() {
  // Authentication state
  const [currentUser, setCurrentUser] = useState<UserInfo | null>(null);
  const [checkingAuth, setCheckingAuth] = useState(true);
  const [showSettings, setShowSettings] = useState(false);

  const [company, setCompany] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [apiKeyConfigured, setApiKeyConfigured] = useState(false);
  const [model, setModel] = useState("claude-sonnet-4-5-20250929");
  const [logs, setLogs] = useState<string[]>([]);
  const [report, setReport] = useState("");
  const [isRunning, setIsRunning] = useState(false);
  const [_showHelp, _setShowHelp] = useState(false); // Reserved for help modal
  const [copyStatus, setCopyStatus] = useState("");
  const [_elapsedTime, _setElapsedTime] = useState(0); // Reserved for future timer feature
  const [streamingOutput, setStreamingOutput] = useState("");

  // Resizable pane state for brief/conversation split
  const [briefPaneHeight, setBriefPaneHeight] = useState(65); // percentage of available height
  const [isResizing, setIsResizing] = useState(false);

  // Manifest and phase state
  const [manifestPath, setManifestPath] = useState<string | null>(null);
  const [manifestId, setManifestId] = useState<string | null>(null);
  const [manifestVersion, setManifestVersion] = useState<string | null>(null);
  const [manifestName, setManifestName] = useState("Default (Fullintel)");
  const [manifestDescription, setManifestDescription] = useState<string | null>(null); // Description shown in main window
  const [manifestInputLabel, setManifestInputLabel] = useState<string | null>(null); // Dynamic placeholder from manifest
  const [savedManifests, setSavedManifests] = useState<SavedManifest[]>([]);
  const [showManifestDropdown, setShowManifestDropdown] = useState(false);
  const [showManifestEditor, setShowManifestEditor] = useState(false);
  const [manifestEditorPath, setManifestEditorPath] = useState<string | null>(null);

  // Follow-up conversation state
  const [followupInput, setFollowupInput] = useState("");
  const [conversation, setConversation] = useState<ConversationMessage[]>([]);
  const [isFollowupRunning, setIsFollowupRunning] = useState(false);

  // Saved briefs state
  const [savedBriefs, setSavedBriefs] = useState<BriefSummary[]>([]);
  const [currentBriefId, setCurrentBriefId] = useState<number | null>(null);
  const [showBriefsList, setShowBriefsList] = useState(false);

  // Custom providers state
  const [customProviders, setCustomProviders] = useState<CustomProviderSummary[]>([]);

  // Research session history state
  const [researchSessions, setResearchSessions] = useState<ResearchSessionSummary[]>([]);
  const [showSessionsList, setShowSessionsList] = useState(false);
  const [selectedSession, setSelectedSession] = useState<ResearchSessionSummary | null>(null);
  const [sessionPhaseOutputs, setSessionPhaseOutputs] = useState<PhaseOutputRecord[]>([]);
  // IM-5041: Track expanded prompt cards (phase_id -> expanded state)
  const [expandedPrompts, setExpandedPrompts] = useState<Set<string>>(new Set());
  // IM-5042: Track session resume state
  const [isResuming, setIsResuming] = useState(false);
  // IM-5043: Track prompt editing state (phase_id -> { systemPrompt, userInput })
  const [editingPrompt, setEditingPrompt] = useState<{
    phaseId: string;
    systemPrompt: string;
    userInput: string;
  } | null>(null);
  // IM-5044: Track phase relaunch state
  const [relaunchingPhase, setRelaunchingPhase] = useState<string | null>(null);
  // IM-5045: Track live prompts during active research
  const [livePhasePrompt, setLivePhasePrompt] = useState<{
    phaseId: string;
    phaseName: string;
    systemPrompt: string | null;
    userInput: string | null;
  } | null>(null);
  const [showLivePrompt, setShowLivePrompt] = useState(false);
  // Track which session's action menu is open (null = none open)
  const [sessionMenuOpen, setSessionMenuOpen] = useState<number | null>(null);
  // Track dropdown position for fixed positioning (escapes overflow:hidden)
  const [menuPosition, setMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  // Track manifest menu open state
  const [manifestMenuOpen, setManifestMenuOpen] = useState(false);

  // Project management state
  const [projects, setProjects] = useState<ProjectSummary[]>([]);
  const [showProjectModal, setShowProjectModal] = useState(false);
  const [newProjectName, setNewProjectName] = useState("");
  const [newProjectDescription, setNewProjectDescription] = useState("");
  const [selectedProject, setSelectedProject] = useState<ProjectSummary | null>(null);
  // Track which projects are expanded in sidebar (show sessions inline)
  const [expandedProjects, setExpandedProjects] = useState<Set<number>>(new Set());
  // Track sessions for each expanded project
  const [projectSessionsMap, setProjectSessionsMap] = useState<Map<number, ResearchSessionSummary[]>>(new Map());
  // Project menu state (similar to session menu)
  const [projectMenuOpen, setProjectMenuOpen] = useState<number | null>(null);
  const [projectMenuPosition, setProjectMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  // Rename project state
  const [renamingProject, setRenamingProject] = useState<number | null>(null);
  const [renameProjectValue, setRenameProjectValue] = useState("");
  // Track which session is being added to a project (for the dropdown)
  const [addToProjectSession, setAddToProjectSession] = useState<ResearchSessionSummary | null>(null);
  const [showAddToProjectDropdown, setShowAddToProjectDropdown] = useState(false);
  // Project session menu state (for ellipsis menu on sessions within a project)
  const [projectSessionMenuOpen, setProjectSessionMenuOpen] = useState<{ projectId: number; sessionId: number } | null>(null);
  const [projectSessionMenuPosition, setProjectSessionMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // Archived items state
  const [showArchived, setShowArchived] = useState(false);
  const [archivedProjects, setArchivedProjects] = useState<ProjectSummary[]>([]);
  const [archivedSessions, setArchivedSessions] = useState<ResearchSessionSummary[]>([]);
  // Archived menu state (for unarchive options)
  const [archivedMenuOpen, setArchivedMenuOpen] = useState<{ type: 'project' | 'session'; id: number } | null>(null);
  const [archivedMenuPosition, setArchivedMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });

  // State for current session panel expansion (ui-preview.html design)
  const [currentSessionExpanded, setCurrentSessionExpanded] = useState(true);

  // Collapsible sidebar sections state
  const [manifestsExpanded, setManifestsExpanded] = useState(true);
  const [projectsExpanded, setProjectsExpanded] = useState(true);
  const [sessionHistoryExpanded, setSessionHistoryExpanded] = useState(true);
  const [todayExpanded, setTodayExpanded] = useState(true);
  const [yesterdayExpanded, setYesterdayExpanded] = useState(true);
  const [previous7DaysExpanded, setPrevious7DaysExpanded] = useState(true);

  // Chat mode state: allows standalone chat without requiring research first
  // When true, input goes directly to chat. When false, input triggers research.
  const [chatMode, setChatMode] = useState(true); // Default to chat-first mode
  // Track if user has explicitly chosen a mode this session
  const [modeExplicitlySet, setModeExplicitlySet] = useState(false);

  // Dynamic phases from manifest
  const [phases, setPhases] = useState<Phase[]>(defaultPhases);
  // Phase navigation: which phase menu is open (null = none)
  const [phaseMenuOpen, setPhaseMenuOpen] = useState<string | null>(null);
  const [phaseMenuPosition, setPhaseMenuPosition] = useState<{ top: number; left: number }>({ top: 0, left: 0 });
  // Ref for scrolling to phase sections in content body
  const contentBodyRef = React.useRef<HTMLDivElement>(null);
  // Ref for auto-resizing textarea
  const chatTextareaRef = React.useRef<HTMLTextAreaElement>(null);

  // Auto-resize textarea handler
  const adjustTextareaHeight = () => {
    const textarea = chatTextareaRef.current;
    if (textarea) {
      textarea.style.height = "auto"; // Reset height to recalculate
      const newHeight = Math.min(textarea.scrollHeight, 150); // Max 150px
      textarea.style.height = `${newHeight}px`;
    }
  };

  // Check if user is logged in on startup
  useEffect(() => {
    const checkAuth = async () => {
      try {
        const user = await invoke<UserInfo | null>("auth_current_user");
        setCurrentUser(user);
      } catch (error) {
        console.log("[DEBUG] No user logged in:", error);
      } finally {
        setCheckingAuth(false);
      }
    };
    checkAuth();
  }, []);

  // Close session menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (sessionMenuOpen !== null) {
        const target = e.target as Element;
        if (!target.closest('.session-menu-wrapper')) {
          setSessionMenuOpen(null);
        }
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, [sessionMenuOpen]);

  // Close manifest menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (manifestMenuOpen) {
        const target = e.target as Element;
        if (!target.closest('.manifest-menu-wrapper')) {
          setManifestMenuOpen(false);
        }
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, [manifestMenuOpen]);

  // Close phase menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (phaseMenuOpen !== null) {
        const target = e.target as Element;
        if (!target.closest('.phase-menu-dropdown') && !target.closest('.header-phase')) {
          setPhaseMenuOpen(null);
        }
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, [phaseMenuOpen]);

  // Close project menu when clicking outside
  useEffect(() => {
    const handleClickOutside = (e: MouseEvent) => {
      if (projectMenuOpen !== null) {
        const target = e.target as Element;
        if (!target.closest('.project-menu-wrapper')) {
          setProjectMenuOpen(null);
        }
      }
      // Also close project session menu
      if (projectSessionMenuOpen !== null) {
        const target = e.target as Element;
        if (!target.closest('.session-menu-dropdown') && !target.closest('.session-menu-btn')) {
          setProjectSessionMenuOpen(null);
        }
      }
      // Also close archived menu
      if (archivedMenuOpen !== null) {
        const target = e.target as Element;
        if (!target.closest('.session-menu-dropdown') && !target.closest('.session-menu-btn')) {
          setArchivedMenuOpen(null);
        }
      }
    };
    document.addEventListener('click', handleClickOutside);
    return () => document.removeEventListener('click', handleClickOutside);
  }, [projectMenuOpen, projectSessionMenuOpen, archivedMenuOpen]);

  // Load manifest input_label when manifest path changes
  useEffect(() => {
    const loadInputLabel = async () => {
      if (!manifestPath) {
        setManifestInputLabel(null);
        return;
      }
      try {
        const label = await invoke<string | null>("get_manifest_input_label", { path: manifestPath });
        setManifestInputLabel(label);
      } catch (error) {
        console.error("Failed to load manifest input label:", error);
        setManifestInputLabel(null);
      }
    };
    loadInputLabel();
  }, [manifestPath]);

  // Load API key for current provider when model changes or user logs in
  useEffect(() => {
    const loadProviderKey = async () => {
      if (!currentUser) {
        setApiKey("");
        setApiKeyConfigured(false);
        return;
      }

      // Check if this is a custom provider (model value starts with "custom_")
      if (model.startsWith("custom_")) {
        try {
          const key = await invoke<string | null>("get_custom_provider_api_key", { providerKey: model });
          if (key) {
            setApiKey(key);
            setApiKeyConfigured(true);
            // Also set in legacy system for compatibility
            await invoke("set_api_key", { key });
          } else {
            // Custom providers might not require an API key (e.g., local Ollama)
            // Check if the provider has a key configured
            const provider = customProviders.find(p => p.provider_key === model);
            if (provider?.has_key) {
              setApiKey("");
              setApiKeyConfigured(false);
            } else {
              // No key required for this custom provider
              setApiKey("");
              setApiKeyConfigured(true); // Allow usage without key
            }
          }
        } catch (error) {
          console.log("[DEBUG] No API key for custom provider:", model, error);
          setApiKey("");
          setApiKeyConfigured(false);
        }
      } else {
        // Built-in provider
        const { provider } = getProviderInfo(model);
        try {
          const key = await invoke<string | null>("get_provider_key", { provider });
          if (key) {
            setApiKey(key);
            setApiKeyConfigured(true);
            // Also set in legacy system for compatibility
            await invoke("set_api_key", { key });
          } else {
            setApiKey("");
            setApiKeyConfigured(false);
          }
        } catch (error) {
          console.log("[DEBUG] No API key for provider:", provider, error);
          setApiKey("");
          setApiKeyConfigured(false);
        }
      }
    };

    loadProviderKey();
  }, [model, currentUser, customProviders]);

  useEffect(() => {
    console.log("[DEBUG] Setting up Tauri event listeners...");

    const unlistenLogs = listen<LogPayload>("agent-log", (event) => {
      console.log("[DEBUG] agent-log received:", event.payload.message);
      setLogs((prev) => [...prev, event.payload.message]);
    });

    const unlistenPhases = listen<PhasePayload>("phase-update", (event) => {
      console.log("[DEBUG] phase-update received:", event.payload);
      setPhases((prev) =>
        prev.map((p) =>
          p.id === event.payload.phase_id
            ? { ...p, status: event.payload.status as Phase["status"] }
            : p
        )
      );
      // Clear streaming output when phase changes
      if (event.payload.status === "running") {
        setStreamingOutput("");
      }
    });

    const unlistenStream = listen<StreamPayload>("stream-token", (event) => {
      setStreamingOutput((prev) => prev + event.payload.token);
    });

    // Listen for phase-output events to persist to SQLite
    // IM-5003: Extended to pass system_prompt and user_input for user data accessibility
    const unlistenPhaseOutput = listen<PhaseOutputPayload>("phase-output", async (event) => {
      const { session_id, phase_id, phase_name, status, system_prompt, user_input, output, error } = event.payload;
      console.log("[DEBUG] phase-output received:", { session_id, phase_id, status, hasPrompt: !!system_prompt });

      // IM-5045: Update live prompt display during active research
      if (status === "running" && (system_prompt || user_input)) {
        setLivePhasePrompt({
          phaseId: phase_id,
          phaseName: phase_name,
          systemPrompt: system_prompt,
          userInput: user_input,
        });
      } else if (status === "completed" || status === "failed") {
        // Clear live prompt when phase completes
        setLivePhasePrompt(null);
      }

      // Only persist if we have a valid session_id
      if (session_id !== null) {
        try {
          await invoke("save_phase_output", {
            sessionId: session_id,
            phaseId: phase_id,
            phaseName: phase_name,
            status,
            systemPrompt: system_prompt,  // IM-5001: Pass system prompt
            userInput: user_input,         // IM-5002: Pass user input
            output,
            error,
          });
          console.log("[DEBUG] Phase output persisted:", phase_id, status);
        } catch (err) {
          console.error("[DEBUG] Failed to persist phase output:", err);
        }
      }
    });

    console.log("[DEBUG] Event listeners registered");

    return () => {
      console.log("[DEBUG] Cleaning up event listeners");
      unlistenLogs.then((f) => f());
      unlistenPhases.then((f) => f());
      unlistenStream.then((f) => f());
      unlistenPhaseOutput.then((f) => f());
    };
  }, []);

  // Load saved manifests and phases on startup
  useEffect(() => {
    const loadInitialData = async () => {
      try {
        // Load saved manifests list
        const manifests = await invoke<SavedManifest[]>("get_saved_manifests");
        setSavedManifests(manifests);

        // Load phases from current manifest
        const phaseInfos = await invoke<PhaseInfo[]>("get_manifest_phases", { manifestPath: null });
        const loadedPhases: Phase[] = phaseInfos.map(p => ({
          id: p.id,
          name: p.name,
          status: "pending"
        }));
        if (loadedPhases.length > 0) {
          setPhases(loadedPhases);
        }
      } catch (error) {
        console.log("[DEBUG] Could not load initial manifest data:", error);
        // Keep default phases if loading fails
      }
    };

    loadInitialData();
  }, []);

  // Load saved briefs when user changes
  useEffect(() => {
    const loadSavedBriefs = async () => {
      if (!currentUser) {
        setSavedBriefs([]);
        return;
      }
      try {
        const briefs = await invoke<BriefSummary[]>("list_briefs");
        setSavedBriefs(briefs);
      } catch (error) {
        console.log("[DEBUG] Could not load saved briefs:", error);
      }
    };

    loadSavedBriefs();
  }, [currentUser]);

  // Load custom providers when user changes
  useEffect(() => {
    const loadCustomProviders = async () => {
      if (!currentUser) {
        setCustomProviders([]);
        return;
      }
      try {
        const providers = await invoke<CustomProviderSummary[]>("list_custom_providers");
        setCustomProviders(providers);
      } catch (error) {
        console.log("[DEBUG] Could not load custom providers:", error);
      }
    };

    loadCustomProviders();
  }, [currentUser]);

  // Load research sessions, projects, and archived items when user changes
  useEffect(() => {
    const loadResearchSessions = async () => {
      if (!currentUser) {
        setResearchSessions([]);
        setProjects([]);
        setArchivedProjects([]);
        setArchivedSessions([]);
        return;
      }
      try {
        const sessions = await invoke<ResearchSessionSummary[]>("list_research_sessions");
        console.log("[DEBUG] Loaded research sessions:", sessions.length, sessions);
        setResearchSessions(sessions);
      } catch (error) {
        console.error("[DEBUG] Could not load research sessions:", error);
      }
      // Also load projects
      try {
        const projectList = await invoke<ProjectSummary[]>("list_projects");
        console.log("[DEBUG] Loaded projects:", projectList.length, projectList);
        setProjects(projectList);
      } catch (error) {
        console.error("[DEBUG] Could not load projects:", error);
      }
      // Also load archived items
      try {
        const [archivedProj, archivedSess] = await Promise.all([
          invoke<ProjectSummary[]>("list_archived_projects"),
          invoke<ResearchSessionSummary[]>("list_archived_sessions"),
        ]);
        console.log("[DEBUG] Loaded archived items:", archivedProj.length, "projects,", archivedSess.length, "sessions");
        setArchivedProjects(archivedProj);
        setArchivedSessions(archivedSess);
      } catch (error) {
        console.error("[DEBUG] Could not load archived items:", error);
      }
    };

    loadResearchSessions();
  }, [currentUser]);

  // Callback for when custom providers change in settings
  const refreshCustomProviders = async () => {
    if (!currentUser) return;
    try {
      const providers = await invoke<CustomProviderSummary[]>("list_custom_providers");
      setCustomProviders(providers);
    } catch (error) {
      console.log("[DEBUG] Could not refresh custom providers:", error);
    }
  };

  // Helper to load phases from a manifest path
  const loadPhasesFromManifest = async (path: string | null) => {
    try {
      const phaseInfos = await invoke<PhaseInfo[]>("get_manifest_phases", { manifestPath: path });
      const loadedPhases: Phase[] = phaseInfos.map(p => ({
        id: p.id,
        name: p.name,
        status: "pending"
      }));
      if (loadedPhases.length > 0) {
        setPhases(loadedPhases);
      }
    } catch (error) {
      console.error("Failed to load phases:", error);
      setPhases(defaultPhases);
    }
  };

  // Reset phases to pending status
  const resetPhases = () => {
    setPhases(prev => prev.map(p => ({ ...p, status: "pending" as const })));
  };

  // Refresh saved briefs list
  const refreshBriefsList = async () => {
    try {
      const briefs = await invoke<BriefSummary[]>("list_briefs");
      setSavedBriefs(briefs);
    } catch (error) {
      console.error("Failed to refresh briefs list:", error);
    }
  };

  // Refresh research sessions list
  const refreshSessionsList = async () => {
    try {
      const sessions = await invoke<ResearchSessionSummary[]>("list_research_sessions");
      setResearchSessions(sessions);
    } catch (error) {
      console.error("Failed to refresh sessions list:", error);
    }
  };

  // Load a session and its phase outputs for viewing
  const viewSession = async (session: ResearchSessionSummary) => {
    try {
      const outputs = await invoke<PhaseOutputRecord[]>("get_phase_outputs", { sessionId: session.id });
      setSelectedSession(session);
      setSessionPhaseOutputs(outputs);
      setShowSessionsList(false);
      setLogs([`📂 Viewing session: ${session.company} (${new Date(session.created_at).toLocaleString()})`]);
    } catch (error) {
      console.error("Failed to load session:", error);
      setLogs(prev => [...prev, `❌ Failed to load session: ${error}`]);
    }
  };

  // Close session detail view
  const closeSessionView = () => {
    setSelectedSession(null);
    setSessionPhaseOutputs([]);
    setExpandedPrompts(new Set()); // Reset expanded prompts when closing
  };

  // IM-5041: Toggle prompt visibility for a phase
  const togglePromptExpanded = (phaseId: string) => {
    setExpandedPrompts(prev => {
      const newSet = new Set(prev);
      if (newSet.has(phaseId)) {
        newSet.delete(phaseId);
      } else {
        newSet.add(phaseId);
      }
      return newSet;
    });
  };

  // IM-5043: Start editing a phase's prompts
  const startEditingPrompt = (output: PhaseOutputRecord) => {
    setEditingPrompt({
      phaseId: output.phase_id,
      systemPrompt: output.system_prompt || "",
      userInput: output.user_input || "",
    });
  };

  // IM-5044: Relaunch a single phase with (optionally edited) prompts
  const relaunchPhase = async (output: PhaseOutputRecord) => {
    if (!apiKey || !selectedSession) {
      setLogs(prev => [...prev, "❌ No API key configured or no session selected."]);
      return;
    }

    // Use edited prompts if available, otherwise use original
    const systemPrompt = editingPrompt?.phaseId === output.phase_id
      ? editingPrompt.systemPrompt
      : output.system_prompt;
    const userInput = editingPrompt?.phaseId === output.phase_id
      ? editingPrompt.userInput
      : output.user_input;

    if (!systemPrompt && !userInput) {
      setLogs(prev => [...prev, "❌ No prompts available to relaunch this phase."]);
      return;
    }

    setRelaunchingPhase(output.phase_id);
    setLogs(prev => [...prev, `🔄 Relaunching phase: ${output.phase_name}...`]);

    try {
      // Ensure API key is configured
      await invoke("set_api_key", { key: apiKey });

      // Call a new backend command to run a single phase with custom prompts
      const result = await invoke<string>("run_single_phase", {
        sessionId: selectedSession.id,
        phaseId: output.phase_id,
        phaseName: output.phase_name,
        systemPrompt: systemPrompt || "",
        userInput: userInput || "",
        model: selectedSession.model,
      });

      // Update the phase output in local state
      setSessionPhaseOutputs(prev =>
        prev.map(p =>
          p.phase_id === output.phase_id
            ? { ...p, output: result, status: "completed", error: null }
            : p
        )
      );

      setLogs(prev => [...prev, `✅ Phase "${output.phase_name}" completed successfully!`]);

      // Clear editing state
      setEditingPrompt(null);
    } catch (error) {
      console.error("Failed to relaunch phase:", error);
      setLogs(prev => [...prev, `❌ Failed to relaunch phase: ${error}`]);

      // Update phase with error
      setSessionPhaseOutputs(prev =>
        prev.map(p =>
          p.phase_id === output.phase_id
            ? { ...p, status: "failed", error: String(error) }
            : p
        )
      );
    } finally {
      setRelaunchingPhase(null);
    }
  };

  // IM-5042: Resume a paused/in_progress session
  const resumeSession = async (session: ResearchSessionSummary) => {
    if (!apiKey) {
      setLogs(prev => [...prev, "❌ No API key configured. Please add one in Settings."]);
      return;
    }

    setIsResuming(true);
    setLogs(prev => [...prev, `🔄 Resuming session for ${session.company}...`]);

    try {
      const result = await invoke<ResumeSessionResult>("resume_research_session", {
        sessionId: session.id
      });

      // Log context info
      setLogs(prev => [
        ...prev,
        `📋 Session context loaded: ${result.context.completed_phases}/${result.context.total_phases} phases completed`,
        `📍 Resuming from phase: ${result.next_phase_id}`,
        `📜 History: ${result.context.history.length} messages loaded`
      ]);

      // Close the session view and start the workflow
      closeSessionView();

      // Set the company name and model from the session
      setCompany(result.session.company);
      setModel(result.session.model);

      // Load manifest if available
      if (result.session.manifest_path) {
        try {
          const content = await invoke<string>("load_manifest_file", { path: result.session.manifest_path });
          const phaseInfos = await invoke<PhaseInfo[]>("validate_manifest", { content });
          setPhases(phaseInfos.map(p => ({ ...p, status: "pending" as const })));
          setManifestPath(result.session.manifest_path);
          setManifestName(result.session.manifest_name || "Loaded Manifest");
          setLogs(prev => [...prev, `• Manifest loaded: ${result.session.manifest_name}`]);
        } catch (manifestError) {
          console.warn("Could not reload manifest:", manifestError);
          setLogs(prev => [...prev, `⚠️ Could not reload manifest, using default phases`]);
        }
      }

      // Note: Session ID tracking happens via the phase-output events
      // TODO: In future, use result.context.history with multi-turn API for seamless resume
      setLogs(prev => [...prev, `✅ Session ready to resume. Click "Generate Brief" to continue.`]);

    } catch (error) {
      console.error("Failed to resume session:", error);
      setLogs(prev => [...prev, `❌ Failed to resume session: ${error}`]);
    } finally {
      setIsResuming(false);
    }
  };

  // Phase navigation: Handle click on phase header item
  const handlePhaseClick = (e: React.MouseEvent, phaseId: string) => {
    e.stopPropagation();
    // Toggle menu or open new one
    if (phaseMenuOpen === phaseId) {
      setPhaseMenuOpen(null);
    } else {
      const rect = (e.target as HTMLElement).getBoundingClientRect();
      setPhaseMenuPosition({ top: rect.bottom + 4, left: rect.left });
      setPhaseMenuOpen(phaseId);
    }
  };

  // Phase navigation: Scroll to phase section in report content
  const scrollToPhase = (phaseName: string) => {
    if (!contentBodyRef.current || !report) return;

    // Search for the phase name in the report text (common patterns like "## Phase Name" or "PHASE: NAME")
    const searchPatterns = [
      `## ${phaseName}`,
      `### ${phaseName}`,
      `# ${phaseName}`,
      `**${phaseName}**`,
      phaseName.toUpperCase(),
      phaseName
    ];

    const preElement = contentBodyRef.current.querySelector('pre');
    if (!preElement) return;

    const text = preElement.textContent || '';

    // Find the position of the phase in the text
    let foundIndex = -1;
    let foundPattern = '';
    for (const pattern of searchPatterns) {
      foundIndex = text.indexOf(pattern);
      if (foundIndex !== -1) {
        foundPattern = pattern;
        break;
      }
    }

    if (foundIndex === -1) {
      setLogs(prev => [...prev, `⚠️ Could not find section for "${phaseName}" in report`]);
      setPhaseMenuOpen(null);
      return;
    }

    // Use DOM Range to find exact pixel position of the text
    try {
      const textNode = preElement.firstChild;
      if (textNode && textNode.nodeType === Node.TEXT_NODE) {
        const range = document.createRange();
        range.setStart(textNode, foundIndex);
        range.setEnd(textNode, foundIndex + foundPattern.length);

        const rect = range.getBoundingClientRect();
        const containerRect = contentBodyRef.current.getBoundingClientRect();

        // Calculate scroll position relative to container
        const scrollTop = contentBodyRef.current.scrollTop + (rect.top - containerRect.top) - 50; // 50px offset for visibility

        contentBodyRef.current.scrollTo({ top: Math.max(0, scrollTop), behavior: 'smooth' });
        setPhaseMenuOpen(null);
        return;
      }
    } catch (e) {
      console.log('[DEBUG] Range-based scroll failed, using fallback:', e);
    }

    // Fallback: Estimate scroll position based on character position
    // Use the contentBodyRef scroll height for accurate scrolling
    const totalLength = text.length;
    const scrollPercentage = foundIndex / totalLength;
    const scrollTop = contentBodyRef.current.scrollHeight * scrollPercentage;

    contentBodyRef.current.scrollTo({ top: scrollTop, behavior: 'smooth' });
    setPhaseMenuOpen(null);
  };

  // Render report with clickable phase headers and table of contents
  const renderReportWithTOC = (reportContent: string) => {
    // Match phase headers: ## Phase Name or separators followed by ## headers
    const phaseRegex = /^##\s+(.+)$/gm;
    const matches: { name: string; index: number }[] = [];
    let match;

    while ((match = phaseRegex.exec(reportContent)) !== null) {
      matches.push({ name: match[1], index: match.index });
    }

    // If no phases found, render as plain text
    if (matches.length === 0) {
      return <pre style={{ whiteSpace: "pre-wrap", fontFamily: "inherit", margin: 0 }}>{reportContent}</pre>;
    }

    // Build table of contents
    const tocItems = matches.map((m, i) => {
      const phaseId = `phase-section-${i}`;
      return (
        <a
          key={phaseId}
          href={`#${phaseId}`}
          onClick={(e) => {
            e.preventDefault();
            const element = document.getElementById(phaseId);
            if (element && contentBodyRef.current) {
              element.scrollIntoView({ behavior: 'smooth', block: 'start' });
            }
          }}
          style={{
            color: 'var(--blue-400)',
            textDecoration: 'none',
            display: 'block',
            padding: '2px 0',
            fontSize: '13px'
          }}
        >
          {m.name}
        </a>
      );
    });

    // Split content into sections and render with anchor IDs
    const sections: JSX.Element[] = [];
    let lastIndex = 0;

    matches.forEach((m, i) => {
      // Add content before this header (if any)
      if (m.index > lastIndex) {
        const beforeContent = reportContent.slice(lastIndex, m.index);
        if (beforeContent.trim()) {
          sections.push(<span key={`pre-${i}`}>{beforeContent}</span>);
        }
      }

      // Find end of this section (start of next header or end of content)
      const nextMatch = matches[i + 1];
      const sectionEnd = nextMatch ? nextMatch.index : reportContent.length;
      const sectionContent = reportContent.slice(m.index, sectionEnd);

      // Render section with clickable header and anchor
      const phaseId = `phase-section-${i}`;
      sections.push(
        <span key={phaseId} id={phaseId} style={{ display: 'block' }}>
          <span
            style={{
              color: 'var(--blue-400)',
              fontWeight: 600,
              cursor: 'pointer',
              display: 'inline-block',
              marginBottom: '4px'
            }}
            onClick={() => {
              const element = document.getElementById(phaseId);
              if (element && contentBodyRef.current) {
                element.scrollIntoView({ behavior: 'smooth', block: 'start' });
              }
            }}
            title="Click to jump to this section"
          >
            ## {m.name}
          </span>
          {sectionContent.slice(sectionContent.indexOf('\n'))}
        </span>
      );

      lastIndex = sectionEnd;
    });

    return (
      <div>
        {/* Table of Contents */}
        <div style={{
          background: 'var(--blue-50)',
          border: '1px solid var(--blue-100)',
          borderRadius: '6px',
          padding: '12px 16px',
          marginBottom: '16px'
        }}>
          <div style={{ fontWeight: 600, marginBottom: '8px', color: 'var(--text-primary)', fontSize: '14px' }}>
            📋 Table of Contents
          </div>
          {tocItems}
        </div>
        {/* Report Content */}
        <pre style={{ whiteSpace: "pre-wrap", fontFamily: "inherit", margin: 0 }}>
          {sections}
        </pre>
      </div>
    );
  };

  // Phase resume: Resume from a specific phase (marks prior phases as complete)
  const resumeFromPhase = async (phaseId: string) => {
    // Find the index of this phase
    const phaseIndex = phases.findIndex(p => p.id === phaseId);
    if (phaseIndex === -1) return;

    // Mark prior phases as completed, this phase and after as pending
    setPhases(prev => prev.map((p, i) => ({
      ...p,
      status: i < phaseIndex ? 'completed' : i === phaseIndex ? 'running' : 'pending'
    })));

    setLogs(prev => [
      ...prev,
      `🔄 Resuming from phase: ${phases[phaseIndex].name}`,
      `📋 ${phaseIndex} phases marked as completed`
    ]);

    // Start research if not already running
    if (!isRunning && company.trim()) {
      // TODO: Implement actual resume from specific phase in backend
      // For now, just indicate readiness
      setLogs(prev => [...prev, `✅ Ready to continue from "${phases[phaseIndex].name}". Click Generate Brief to proceed.`]);
    }

    setPhaseMenuOpen(null);
  };

  // Helper to perform file actions on a session (copy, print, save) from menu
  // Fetches outputs first, then performs action without changing view
  const sessionFileAction = async (
    session: ResearchSessionSummary,
    action: "copy" | "print" | "save"
  ) => {
    try {
      // Fetch the session's phase outputs
      const outputs = await invoke<PhaseOutputRecord[]>("get_phase_outputs", { sessionId: session.id });

      if (outputs.length === 0) {
        setLogs(prev => [...prev, `⚠️ No outputs to ${action} for this session`]);
        return;
      }

      // Perform the action with the fetched data
      switch (action) {
        case "copy":
          await handleCopy(session, outputs);
          setLogs(prev => [...prev, `📋 Session "${session.company}" copied to clipboard`]);
          break;
        case "print":
          handlePrint(session, outputs);
          break;
        case "save":
          await handleSave(session, outputs);
          break;
      }
    } catch (error) {
      console.error(`Failed to ${action} session:`, error);
      setLogs(prev => [...prev, `❌ Failed to ${action} session: ${error}`]);
    }
  };

  // Delete a research session
  const deleteSession = async (sessionId: number) => {
    if (!confirm("Are you sure you want to delete this session? This will also delete all phase outputs.")) {
      return;
    }
    try {
      await invoke("delete_research_session", { sessionId });
      await refreshSessionsList();
      if (selectedSession?.id === sessionId) {
        closeSessionView();
      }
      setLogs(prev => [...prev, "🗑️ Session deleted"]);
    } catch (error) {
      console.error("Failed to delete session:", error);
      setLogs(prev => [...prev, `❌ Failed to delete session: ${error}`]);
    }
  };

  // Export a research session to Documents folder as Markdown files
  const exportSession = async (sessionId: number, sessionCompany: string) => {
    try {
      setLogs(prev => [...prev, `• Exporting session for ${sessionCompany}...`]);

      // Export entire session as a combined Markdown file
      const exportPath = await invoke<string>("export_session_as_markdown", { sessionId });

      setLogs(prev => [...prev, `✅ Session exported to: ${exportPath}`]);

      // Also export individual phases
      const phasePaths = await invoke<string[]>("export_all_phases_as_markdown", { sessionId });

      if (phasePaths.length > 0) {
        setLogs(prev => [...prev, `📁 ${phasePaths.length} phase files created in same directory`]);
      }
    } catch (error) {
      console.error("Failed to export session:", error);
      setLogs(prev => [...prev, `❌ Failed to export session: ${error}`]);
    }
  };

  // ========== PROJECT MANAGEMENT FUNCTIONS ==========

  // Load all projects for the current user
  const loadProjects = async () => {
    try {
      const projectList = await invoke<ProjectSummary[]>("list_projects");
      setProjects(projectList);
    } catch (error) {
      console.error("Failed to load projects:", error);
    }
  };

  // Load archived items (projects and sessions)
  const loadArchivedItems = async () => {
    try {
      const [archivedProj, archivedSess] = await Promise.all([
        invoke<ProjectSummary[]>("list_archived_projects"),
        invoke<ResearchSessionSummary[]>("list_archived_sessions"),
      ]);
      setArchivedProjects(archivedProj);
      setArchivedSessions(archivedSess);
    } catch (error) {
      console.error("Failed to load archived items:", error);
    }
  };

  // Create a new project
  const createProject = async () => {
    if (!newProjectName.trim()) {
      setLogs(prev => [...prev, "❌ Project name is required"]);
      return;
    }
    try {
      const projectId = await invoke<number>("create_project", {
        name: newProjectName.trim(),
        description: newProjectDescription.trim() || null
      });
      setLogs(prev => [...prev, `📁 Project "${newProjectName}" created (ID: ${projectId})`]);
      setShowProjectModal(false);
      setNewProjectName("");
      setNewProjectDescription("");
      await loadProjects();
    } catch (error) {
      console.error("Failed to create project:", error);
      setLogs(prev => [...prev, `❌ Failed to create project: ${error}`]);
    }
  };

  // Delete a project
  const deleteProject = async (projectId: number, projectName: string) => {
    if (!confirm(`Are you sure you want to delete project "${projectName}"? Sessions will be unlinked but not deleted.`)) {
      return;
    }
    try {
      await invoke("delete_project", { projectId });
      setLogs(prev => [...prev, `🗑️ Project "${projectName}" deleted`]);
      if (selectedProject?.id === projectId) {
        setSelectedProject(null);
      }
      // Remove from expanded set
      setExpandedProjects(prev => {
        const next = new Set(prev);
        next.delete(projectId);
        return next;
      });
      setProjectMenuOpen(null);
      await loadProjects();
    } catch (error) {
      console.error("Failed to delete project:", error);
      setLogs(prev => [...prev, `❌ Failed to delete project: ${error}`]);
    }
  };

  // Toggle project expansion (show/hide sessions inline)
  const toggleProjectExpansion = async (project: ProjectSummary) => {
    const isExpanded = expandedProjects.has(project.id);
    if (isExpanded) {
      // Collapse
      setExpandedProjects(prev => {
        const next = new Set(prev);
        next.delete(project.id);
        return next;
      });
    } else {
      // Expand - load sessions first
      try {
        const sessions = await invoke<ResearchSessionSummary[]>("get_project_sessions", {
          projectId: project.id
        });
        setProjectSessionsMap(prev => {
          const next = new Map(prev);
          next.set(project.id, sessions);
          return next;
        });
        setExpandedProjects(prev => new Set(prev).add(project.id));
      } catch (error) {
        console.error("Failed to load project sessions:", error);
        setLogs(prev => [...prev, `❌ Failed to load project sessions: ${error}`]);
      }
    }
  };

  // Open project menu (ellipsis)
  const openProjectMenu = (e: React.MouseEvent, projectId: number) => {
    e.stopPropagation();
    const rect = (e.target as HTMLElement).getBoundingClientRect();
    setProjectMenuPosition({ top: rect.bottom + 4, left: rect.left });
    setProjectMenuOpen(projectId);
  };

  // Start renaming a project
  const startRenameProject = (project: ProjectSummary) => {
    setRenamingProject(project.id);
    setRenameProjectValue(project.name);
    setProjectMenuOpen(null);
  };

  // Save renamed project
  const saveRenameProject = async (projectId: number) => {
    if (!renameProjectValue.trim()) {
      setRenamingProject(null);
      return;
    }
    try {
      await invoke("update_project", {
        projectId,
        name: renameProjectValue.trim(),
        description: null // Keep existing description
      });
      setLogs(prev => [...prev, `✏️ Project renamed to "${renameProjectValue.trim()}"`]);
      setRenamingProject(null);
      await loadProjects();
    } catch (error) {
      console.error("Failed to rename project:", error);
      setLogs(prev => [...prev, `❌ Failed to rename project: ${error}`]);
    }
  };

  // Refresh sessions for an expanded project
  const refreshProjectSessions = async (projectId: number) => {
    try {
      const sessions = await invoke<ResearchSessionSummary[]>("get_project_sessions", { projectId });
      setProjectSessionsMap(prev => {
        const next = new Map(prev);
        next.set(projectId, sessions);
        return next;
      });
    } catch (error) {
      console.error("Failed to refresh project sessions:", error);
    }
  };

  // Add a session to a project (also renames session to include project name)
  const addSessionToProject = async (projectId: number, sessionId: number) => {
    try {
      const project = projects.find(p => p.id === projectId);
      const session = addToProjectSession;

      // Add session to project
      await invoke("add_session_to_project", { projectId, sessionId });

      // Rename session to format: "session-name-project-name"
      if (project && session) {
        const newName = `${session.company}-${project.name}`;
        try {
          await invoke("rename_research_session", { sessionId, newName });
        } catch (renameError) {
          console.log("Session rename not available yet:", renameError);
        }
      }

      setLogs(prev => [...prev, `✅ Session added to project "${project?.name || projectId}"`]);
      setShowAddToProjectDropdown(false);
      setAddToProjectSession(null);
      await loadProjects();
      await refreshSessionsList(); // Reload sessions to show new name
      // Refresh expanded project sessions
      if (expandedProjects.has(projectId)) {
        await refreshProjectSessions(projectId);
      }
    } catch (error) {
      console.error("Failed to add session to project:", error);
      setLogs(prev => [...prev, `❌ Failed to add session to project: ${error}`]);
    }
  };

  // Remove a session from a project
  const removeSessionFromProject = async (projectId: number, sessionId: number) => {
    try {
      await invoke("remove_session_from_project", { projectId, sessionId });
      setLogs(prev => [...prev, "✅ Session removed from project"]);
      // Refresh expanded project sessions
      if (expandedProjects.has(projectId)) {
        await refreshProjectSessions(projectId);
      }
      await loadProjects();
    } catch (error) {
      console.error("Failed to remove session from project:", error);
      setLogs(prev => [...prev, `❌ Failed to remove session from project: ${error}`]);
    }
  };

  // Get status badge color (handles both 'running' from DB and 'in_progress' for display)
  const getStatusBadgeClass = (status: string): string => {
    switch (status) {
      case "completed": return "status-completed";
      case "in_progress":
      case "running": return "status-running";
      case "failed": return "status-failed";
      default: return "status-pending";
    }
  };

  // Load a saved brief
  const loadBrief = async (briefId: number) => {
    try {
      const brief = await invoke<Brief | null>("get_brief", { briefId });
      if (brief) {
        setCompany(brief.company);
        setModel(brief.model);
        setReport(brief.content);
        setCurrentBriefId(brief.id);
        setLogs([`📂 Loaded saved brief: ${brief.company} (${new Date(brief.created_at).toLocaleString()})`]);

        // Load associated conversation
        const messages = await invoke<PersistedConversationMessage[]>("get_conversation", { briefId });
        const loadedConversation: ConversationMessage[] = messages.map(m => ({
          role: m.role as "user" | "assistant",
          content: m.content,
        }));
        setConversation(loadedConversation);
        setShowBriefsList(false);
      }
    } catch (error) {
      console.error("Failed to load brief:", error);
      setLogs(prev => [...prev, `❌ Failed to load brief: ${error}`]);
    }
  };

  // Delete a saved brief
  const deleteBrief = async (briefId: number) => {
    if (!confirm("Are you sure you want to delete this brief? This will also delete all associated conversations.")) {
      return;
    }
    try {
      await invoke<boolean>("delete_brief", { briefId });
      await refreshBriefsList();
      if (currentBriefId === briefId) {
        setCurrentBriefId(null);
        handleReset();
      }
      setLogs(prev => [...prev, "🗑️ Brief deleted"]);
    } catch (error) {
      console.error("Failed to delete brief:", error);
      setLogs(prev => [...prev, `❌ Failed to delete brief: ${error}`]);
    }
  };

  // Reset everything for a new search
  const handleReset = () => {
    setCompany("");
    setReport("");
    setLogs([]);
    resetPhases();
    setCopyStatus("");
    setConversation([]);
    setCurrentBriefId(null);
    // Close any open historical session view
    setSelectedSession(null);
    setSessionPhaseOutputs([]);
  };

  // Abort running operation and reset
  const handleAbort = () => {
    setIsRunning(false);
    setLogs((prev) => [...prev, "⚠️ Operation aborted by user"]);
    setPhases((prev) =>
      prev.map((p) =>
        p.status === "running" ? { ...p, status: "failed" } : p
      )
    );
  };

  // Copy report or session output to clipboard
  const handleCopy = async (sessionOverride?: ResearchSessionSummary, outputsOverride?: PhaseOutputRecord[]) => {
    let contentToCopy = "";

    // Use overrides if provided (for menu actions), otherwise use current state
    const session = sessionOverride || selectedSession;
    const outputs = outputsOverride || sessionPhaseOutputs;

    // If viewing a session, copy all phase outputs
    if (session && outputs.length > 0) {
      contentToCopy = outputs
        .filter(p => p.output)
        .map(p => `## ${p.phase_name}\n\n${p.output}`)
        .join("\n\n---\n\n");
    } else if (report) {
      contentToCopy = report;
    }

    if (!contentToCopy) return;

    try {
      await navigator.clipboard.writeText(contentToCopy);
      setCopyStatus("Copied");
      setTimeout(() => setCopyStatus(""), 2000);
    } catch {
      setCopyStatus("Failed to copy");
    }
  };

  // Print the report or session - creates printable view in current window
  const handlePrint = (sessionOverride?: ResearchSessionSummary, outputsOverride?: PhaseOutputRecord[]) => {
    // Use overrides if provided (for menu actions), otherwise use current state
    const session = sessionOverride || selectedSession;
    const outputs = outputsOverride || sessionPhaseOutputs;

    // Check if there's anything to print
    const hasContent = report || (session && outputs.length > 0) || logs.length > 0;
    if (!hasContent) return;

    // Create a hidden iframe for printing
    const iframe = document.createElement("iframe");
    iframe.style.position = "absolute";
    iframe.style.top = "-10000px";
    document.body.appendChild(iframe);

    const targetName = session ? session.company : company;

    // Build content based on what we're viewing
    let contentHtml = "";

    if (session && outputs.length > 0) {
      // Session view - print all phase outputs
      contentHtml = outputs
        .filter(p => p.output)
        .map(p => `<h2>${p.phase_name}</h2><pre style="white-space:pre-wrap;font-family:inherit;">${p.output}</pre>`)
        .join("<hr/>");
    } else {
      const processLogHtml = logs.length > 0
        ? `<h2>Process Log</h2><pre style="background:#f5f5f5;padding:15px;font-size:11px;">${logs.join("\n")}</pre>`
        : "";

      const reportHtml = report
        ? `<h2>Opportunity Brief</h2><pre style="white-space:pre-wrap;font-family:inherit;">${report}</pre>`
        : "";

      contentHtml = processLogHtml + reportHtml;
    }

    iframe.contentDocument?.write(`
      <html>
        <head>
          <title>Fullintel Agent - ${targetName}</title>
          <style>
            body { font-family: Arial, sans-serif; padding: 40px; line-height: 1.6; }
            h1 { color: #333; border-bottom: 2px solid #007bff; padding-bottom: 10px; }
            h2 { color: #555; margin-top: 30px; }
            pre { white-space: pre-wrap; }
            hr { border: none; border-top: 1px solid #ddd; margin: 30px 0; }
          </style>
        </head>
        <body>
          <h1>Fullintel Agent Output: ${targetName}</h1>
          <p><strong>Generated:</strong> ${new Date().toLocaleString()}</p>
          ${contentHtml}
        </body>
      </html>
    `);
    iframe.contentDocument?.close();

    iframe.contentWindow?.focus();
    iframe.contentWindow?.print();

    // Clean up after printing
    setTimeout(() => document.body.removeChild(iframe), 1000);
  };

  // Save report or session to file using Tauri native dialog
  const handleSave = async (sessionOverride?: ResearchSessionSummary, outputsOverride?: PhaseOutputRecord[]) => {
    // Use overrides if provided (for menu actions), otherwise use current state
    const session = sessionOverride || selectedSession;
    const outputs = outputsOverride || sessionPhaseOutputs;

    // Check if there's anything to save
    const hasContent = report || (session && outputs.length > 0) || logs.length > 0;
    if (!hasContent) return;

    const targetName = session ? session.company : company;

    // Build full content based on what we're viewing
    let processLog = "";
    let reportContent = "";

    if (session && outputs.length > 0) {
      // Session view - save all phase outputs
      reportContent = `RESEARCH SESSION: ${targetName}\n${"=".repeat(50)}\n\n` +
        outputs
          .filter(p => p.output)
          .map(p => `## ${p.phase_name}\n${"─".repeat(40)}\n\n${p.output}`)
          .join("\n\n");
    } else {
      processLog = logs.length > 0
        ? `PROCESS LOG\n${"=".repeat(50)}\n${logs.join("\n")}\n\n`
        : "";

      reportContent = report
        ? `OPPORTUNITY BRIEF: ${targetName}\n${"=".repeat(50)}\n\n${report}`
        : "";
    }

    // Build user info string
    const userName = currentUser?.first_name || currentUser?.last_name
      ? `${currentUser.first_name || ""} ${currentUser.last_name || ""}`.trim()
      : currentUser?.username || "Unknown";
    const userRole = currentUser?.role ? ` (${currentUser.role})` : "";
    const userLocation = currentUser?.location ? ` - ${currentUser.location}` : "";

    // Get provider info for header
    const providerDetails = getProviderInfoWithCustom(model, customProviders);

    const fullContent = `FULLINTEL AGENT OUTPUT
${"=".repeat(60)}
Generated:  ${new Date().toLocaleString()}
Author:     ${userName}${userRole}${userLocation}
Target:     ${targetName}
Provider:   ${providerDetails.name} (${session ? session.model : model})
Manifest:   ${session?.manifest_name || manifestName}
${"=".repeat(60)}

${processLog}${reportContent}
`;

    // Build filename with provider info for easy differentiation
    const sanitizedCompany = targetName.replace(/\s+/g, "_").replace(/[^a-zA-Z0-9_-]/g, "");
    const sanitizedProvider = providerDetails.name.replace(/\s+/g, "-").toLowerCase();
    const dateStr = new Date().toISOString().split("T")[0];
    const defaultFilename = `${sanitizedCompany}_${sanitizedProvider}_brief_${dateStr}.txt`;

    try {
      const filePath = await save({
        defaultPath: defaultFilename,
        filters: [{ name: "Text Files", extensions: ["txt", "md"] }]
      });

      if (filePath) {
        await writeTextFile(filePath, fullContent);
        setCopyStatus("Saved");
        setTimeout(() => setCopyStatus(""), 2000);
      }
    } catch (error) {
      console.error("Save failed:", error);
      setCopyStatus("Save failed");
    }
  };

  // Conversation-specific action handlers
  const [conversationCopyStatus, setConversationCopyStatus] = useState("");

  // Format conversation to text
  const formatConversationToText = (): string => {
    if (conversation.length === 0) return "";

    let text = "=".repeat(60) + "\n";
    text += `CONVERSATION LOG\n`;
    text += `Subject: ${company || "General"}\n`;
    text += `Model: ${model}\n`;
    text += `Date: ${new Date().toLocaleString()}\n`;
    text += "=".repeat(60) + "\n\n";

    for (const msg of conversation) {
      const role = msg.role === "user" ? "YOU" : "ASSISTANT";
      text += `--- ${role} ---\n`;
      text += msg.content + "\n\n";
    }

    return text;
  };

  // Copy conversation to clipboard
  const handleCopyConversation = async () => {
    const content = formatConversationToText();
    if (!content) return;

    try {
      await navigator.clipboard.writeText(content);
      setConversationCopyStatus("Copied");
      setTimeout(() => setConversationCopyStatus(""), 2000);
    } catch {
      setConversationCopyStatus("Failed");
    }
  };

  // Save conversation to file
  const handleSaveConversation = async () => {
    const content = formatConversationToText();
    if (!content) return;

    try {
      const defaultFilename = `conversation_${company || "general"}_${new Date().toISOString().split("T")[0]}.txt`;

      const filePath = await save({
        defaultPath: defaultFilename,
        filters: [{ name: "Text Files", extensions: ["txt", "md"] }]
      });

      if (filePath) {
        await writeTextFile(filePath, content);
        setConversationCopyStatus("Saved");
        setTimeout(() => setConversationCopyStatus(""), 2000);
      }
    } catch (error) {
      console.error("Save conversation failed:", error);
      setConversationCopyStatus("Save failed");
    }
  };

  // Extract YAML manifest from conversation and save it
  // Looks for YAML code blocks in assistant messages
  const handleSaveAsManifest = async () => {
    // Look for YAML content in assistant messages
    let yamlContent = "";

    for (const msg of conversation) {
      if (msg.role === "assistant") {
        // Look for YAML code blocks (```yaml ... ```)
        const yamlMatch = msg.content.match(/```(?:yaml|yml)\s*([\s\S]*?)```/i);
        if (yamlMatch) {
          yamlContent = yamlMatch[1].trim();
          break;
        }
        // Also check for raw YAML that starts with "manifest:" or "phases:"
        if (msg.content.includes("manifest:") || msg.content.includes("phases:")) {
          // Try to extract the YAML portion
          const lines = msg.content.split("\n");
          const yamlStartIdx = lines.findIndex(l =>
            l.trim().startsWith("manifest:") ||
            l.trim().startsWith("phases:") ||
            l.trim().startsWith("schemas:")
          );
          if (yamlStartIdx !== -1) {
            yamlContent = lines.slice(yamlStartIdx).join("\n").trim();
            break;
          }
        }
      }
    }

    if (!yamlContent) {
      alert("No manifest YAML found in conversation. Ask the assistant to generate a manifest template in YAML format.");
      return;
    }

    try {
      // Validate the YAML first
      const phases = await invoke<PhaseInfo[]>("validate_manifest", { content: yamlContent });

      if (phases.length === 0) {
        alert("The YAML doesn't appear to be a valid manifest (no phases found).");
        return;
      }

      // Save the manifest
      const defaultFilename = `new_manifest_${new Date().toISOString().split("T")[0]}.yaml`;

      const filePath = await save({
        defaultPath: defaultFilename,
        filters: [{ name: "YAML Files", extensions: ["yaml", "yml"] }]
      });

      if (filePath) {
        await invoke("save_manifest_file", { path: filePath, content: yamlContent });

        // Ask if user wants to add to saved manifests
        if (confirm(`Manifest saved! It has ${phases.length} phases. Add to your saved manifests?`)) {
          const name = prompt("Enter a name for this manifest:", phases[0]?.name || "New Manifest") || "New Manifest";
          await invoke("save_manifest_to_list", { name, path: filePath });
          // Refresh the saved manifests list
          const manifests = await invoke<SavedManifest[]>("get_saved_manifests");
          setSavedManifests(manifests);
        }

        setConversationCopyStatus("Manifest saved!");
        setTimeout(() => setConversationCopyStatus(""), 3000);
      }
    } catch (error) {
      console.error("Save as manifest failed:", error);
      alert(`Failed to save manifest: ${error}`);
    }
  };

  // Resizable pane handlers
  const splitContainerRef = React.useRef<HTMLDivElement>(null);

  const handleResizeStart = (e: React.MouseEvent) => {
    e.preventDefault();
    setIsResizing(true);
  };

  const handleResizeMove = React.useCallback((e: MouseEvent) => {
    if (!isResizing || !splitContainerRef.current) return;

    const container = splitContainerRef.current;
    const containerRect = container.getBoundingClientRect();
    const containerHeight = containerRect.height;
    const mouseY = e.clientY - containerRect.top;

    // Calculate percentage (clamp between 20% and 80%)
    let newHeight = (mouseY / containerHeight) * 100;
    newHeight = Math.max(20, Math.min(80, newHeight));

    setBriefPaneHeight(newHeight);
  }, [isResizing]);

  const handleResizeEnd = React.useCallback(() => {
    setIsResizing(false);
  }, []);

  // Add/remove mouse event listeners for resizing
  React.useEffect(() => {
    if (isResizing) {
      document.addEventListener("mousemove", handleResizeMove);
      document.addEventListener("mouseup", handleResizeEnd);
      document.body.style.cursor = "row-resize";
      document.body.style.userSelect = "none";
    } else {
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    }

    return () => {
      document.removeEventListener("mousemove", handleResizeMove);
      document.removeEventListener("mouseup", handleResizeEnd);
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
    };
  }, [isResizing, handleResizeMove, handleResizeEnd]);

  // Load a custom manifest file via file dialog
  const handleLoadManifest = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "YAML Files", extensions: ["yaml", "yml"] }]
      });

      if (selected && typeof selected === "string") {
        await invoke("set_manifest_path", { path: selected });
        setManifestPath(selected);

        // Get manifest info from YAML content (id, version, name, description, input_label)
        const manifestInfo = await invoke<{ id: string; version: string; name: string; description: string; input_label: string | null }>("get_manifest_info", { manifestPath: selected });
        setManifestId(manifestInfo.id);
        setManifestVersion(manifestInfo.version);
        setManifestName(manifestInfo.name);
        setManifestDescription(manifestInfo.description);
        setManifestInputLabel(manifestInfo.input_label);

        // Load phases from the new manifest
        await loadPhasesFromManifest(selected);

        // Save to manifest list with the proper name from YAML
        await invoke("save_manifest_to_list", { name: manifestInfo.name, path: selected });

        // Refresh saved manifests
        const manifests = await invoke<SavedManifest[]>("get_saved_manifests");
        setSavedManifests(manifests);

        setLogs((prev) => [...prev, `• Loaded manifest: ${manifestInfo.name}`]);
      }
    } catch (error) {
      console.error("Failed to load manifest:", error);
      setLogs((prev) => [...prev, `❌ Failed to load manifest: ${error}`]);
    }
  };

  // Open manifest editor with existing manifest
  const handleEditManifest = (path: string | null) => {
    setManifestEditorPath(path);
    setShowManifestEditor(true);
  };

  // Open manifest editor to create new manifest
  const handleNewManifest = () => {
    setManifestEditorPath(null);
    setShowManifestEditor(true);
  };

  // Callback when manifest is loaded from editor
  const handleManifestLoaded = async (path: string, name: string, phases: PhaseInfo[]) => {
    setManifestPath(path);
    setManifestName(name);
    setPhases(phases.map(p => ({ ...p, status: "pending" as const })));

    // Update backend
    await invoke("set_manifest_path", { path });

    // Fetch full manifest info for id, version, description and input label
    try {
      const manifestInfo = await invoke<{ id: string; version: string; name: string; description: string; input_label: string | null }>("get_manifest_info", { manifestPath: path });
      setManifestId(manifestInfo.id);
      setManifestVersion(manifestInfo.version);
      setManifestDescription(manifestInfo.description);
      setManifestInputLabel(manifestInfo.input_label);
    } catch (e) {
      console.log("[DEBUG] Could not load manifest description:", e);
    }

    // Refresh saved manifests list
    const manifests = await invoke<SavedManifest[]>("get_saved_manifests");
    setSavedManifests(manifests);

    setLogs((prev) => [...prev, `• Using manifest: ${name}`]);
  };

  // Callback when manifest is saved from editor
  const handleManifestSaved = async (_path: string, _name: string) => {
    // Refresh saved manifests list
    const manifests = await invoke<SavedManifest[]>("get_saved_manifests");
    setSavedManifests(manifests);
  };

  // Select a saved manifest from dropdown
  const handleSelectManifest = async (manifest: SavedManifest) => {
    try {
      await invoke("set_manifest_path", { path: manifest.path });
      setManifestPath(manifest.path);
      // Get manifest info (id, version, name, description, input_label)
      const manifestInfo = await invoke<{ id: string; version: string; name: string; description: string; input_label: string | null }>("get_manifest_info", { manifestPath: manifest.path });
      setManifestId(manifestInfo.id);
      setManifestVersion(manifestInfo.version);
      setManifestName(manifestInfo.name);
      setManifestDescription(manifestInfo.description);
      setManifestInputLabel(manifestInfo.input_label);
      await loadPhasesFromManifest(manifest.path);
      setShowManifestDropdown(false);
      setLogs((prev) => [...prev, `• Selected manifest: ${manifestInfo.name}`]);
    } catch (error) {
      console.error("Failed to select manifest:", error);
      setLogs((prev) => [...prev, `❌ Failed to select manifest: ${error}`]);
    }
  };

  // Send a chat message or follow-up question
  // Works in both standalone chat mode and post-research followup mode
  const handleFollowup = async () => {
    if (!followupInput.trim()) return;

    // Check API key is set
    if (!apiKeyConfigured || !apiKey) {
      alert("Please configure an API Key in Settings first.");
      setShowSettings(true);
      return;
    }

    const question = followupInput.trim();
    setFollowupInput("");
    setIsFollowupRunning(true);

    // Add user question to conversation
    setConversation((prev) => [...prev, { role: "user", content: question }]);

    try {
      // Ensure API key is configured
      await invoke("set_api_key", { key: apiKey });

      // Build context from report (if available) and conversation history
      let context = "";
      if (report) {
        context = `Generated Report:\n${report}\n\n`;
      }
      if (conversation.length > 0) {
        context += "Conversation History:\n" + conversation.map(m =>
          `${m.role === "user" ? "User" : "Assistant"}: ${m.content}`
        ).join("\n\n");
      }
      if (!context) {
        // Standalone chat mode - provide helpful context about the app
        context = `This is a new conversation. The user is using the Fullintel Agent, a research and intelligence platform.
You can help with:
- Answering questions about companies, industries, or topics
- Creating research manifests/templates for structured analysis
- General conversation and assistance
- Providing analysis and insights

If the user wants to create a new research manifest/template, help them design the phases and structure.`;
      }

      const response = await invoke<string>("send_followup", {
        question,
        context,
        model,
      });

      setConversation((prev) => [...prev, { role: "assistant", content: response }]);

      // Persist conversation to database if we have a current brief
      if (currentBriefId) {
        try {
          await invoke("add_conversation_message", {
            briefId: currentBriefId,
            role: "user",
            content: question,
          });
          await invoke("add_conversation_message", {
            briefId: currentBriefId,
            role: "assistant",
            content: response,
          });
        } catch (persistError) {
          console.error("Failed to persist conversation:", persistError);
        }
      }
    } catch (error) {
      console.error("Chat failed:", error);
      setConversation((prev) => [...prev, { role: "assistant", content: `Error: ${error}` }]);
    } finally {
      setIsFollowupRunning(false);
    }
  };

  async function startResearch() {
    if (!apiKeyConfigured || !apiKey) {
      alert("Please configure an API Key in Settings first.");
      setShowSettings(true);
      return;
    }
    if (!company.trim()) {
      alert("Please enter a target company.");
      return;
    }

    setIsRunning(true);
    setStreamingOutput("");
    setReport("");
    resetPhases(); // Reset phases to pending status
    setConversation([]); // Clear previous conversation
    setCurrentBriefId(null); // Clear current brief since this is a new research

    // Immediate feedback - these show right away
    const startLogs = [
      `▶ Starting research for "${company}"`,
      `• Model: ${model}`,
      `• Manifest: ${manifestName}`,
      `🔑 Configuring API key...`,
    ];
    setLogs(startLogs);

    try {
      await invoke("set_api_key", { key: apiKey });
      setLogs((prev) => [...prev, "✅ API key configured"]);
      setLogs((prev) => [...prev, "📤 Sending to agent..."]);

      // Start research - session is created at the start of run_research
      // Use Promise to allow UI update while research runs
      const researchPromise = invoke<string>("run_research", {
        company,
        model,
        manifestPathOverride: manifestPath,
      });

      // Give the backend a moment to create the session, then refresh the list
      // This allows users to see the running session in the sidebar
      setTimeout(async () => {
        await refreshSessionsList();
        console.log("[DEBUG] Sessions refreshed after research start");
      }, 500);

      const result = await researchPromise;
      setReport(result);
      setStreamingOutput(""); // Clear streaming output when report is ready to prevent duplication
      setLogs((prev) => [...prev, "✅ Research completed successfully!"]);

      // Save brief to database
      try {
        const briefId = await invoke<number>("save_brief", {
          company: company.trim(),
          model,
          manifestName: manifestName !== "Default (Fullintel)" ? manifestName : null,
          content: result,
        });
        setCurrentBriefId(briefId);
        await refreshBriefsList();
        await refreshSessionsList(); // Refresh sessions after research completes
        setLogs((prev) => [...prev, "💾 Brief saved to history"]);
      } catch (saveError) {
        console.error("Failed to save brief:", saveError);
        setLogs((prev) => [...prev, `⚠️ Brief not saved: ${saveError}`]);
      }
    } catch (error) {
      console.error(error);
      setLogs((prev) => [...prev, `❌ Error: ${error}`]);
      await refreshSessionsList(); // Also refresh on error to show failed session
    } finally {
      setIsRunning(false);
    }
  }

  const providerInfo = getProviderInfoWithCustom(model, customProviders);

  // Handle logout
  const handleLogout = () => {
    setCurrentUser(null);
    setApiKey("");
    setApiKeyConfigured(false);
  };

  // Show loading while checking auth
  if (checkingAuth) {
    return (
      <div className="container" style={{ justifyContent: "center", alignItems: "center" }}>
        <div style={{ textAlign: "center", color: "#666" }}>
          <h2>Loading...</h2>
        </div>
      </div>
    );
  }

  // Show login screen if not authenticated
  if (!currentUser) {
    return <AuthScreen onLoginSuccess={setCurrentUser} />;
  }

  return (
    <div className="app">
      {/* Settings Panel Modal */}
      {showSettings && (
        <SettingsPanel
          user={currentUser}
          onClose={() => setShowSettings(false)}
          onLogout={handleLogout}
          onCustomProvidersChange={refreshCustomProviders}
          onUserUpdate={setCurrentUser}
        />
      )}

      {/* Manifest Editor Modal */}
      <ManifestEditor
        isOpen={showManifestEditor}
        onClose={() => setShowManifestEditor(false)}
        currentManifestPath={manifestEditorPath}
        onManifestLoaded={handleManifestLoaded}
        onManifestSaved={handleManifestSaved}
        onRemoveManifest={async (path) => {
          try {
            await invoke("remove_saved_manifest", { path });
            const manifests = await invoke<SavedManifest[]>("get_saved_manifests");
            setSavedManifests(manifests);
            // If we removed the currently selected manifest, reset to default
            if (path === manifestPath) {
              setManifestPath(null);
              setManifestName("Default (Fullintel)");
              setManifestDescription(null);
              setManifestInputLabel(null);
            }
          } catch (error) {
            console.error("Failed to remove manifest:", error);
          }
        }}
      />

      {/* LEFT SIDEBAR */}
      <div className="sidebar">
        {/* MANIFEST TEMPLATES SECTION - Collapsible */}
        <div className="sidebar-section" style={{ paddingTop: "8px" }}>
          <div className="section-label" style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}>
            <span
              onClick={() => setManifestsExpanded(!manifestsExpanded)}
              style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px" }}
            >
              <span style={{ fontSize: "10px", minWidth: "12px" }}>{manifestsExpanded ? "▼" : "▶"}</span>
              Manifest Templates
            </span>
            <div className="manifest-menu-wrapper">
              <button
                className="manifest-menu-btn"
                onClick={() => setManifestMenuOpen(!manifestMenuOpen)}
                title="Manifest options"
              >
                ⋯
              </button>
              {manifestMenuOpen && (
                <div className="manifest-menu-dropdown">
                  <button
                    className="manifest-menu-item"
                    onClick={() => { handleNewManifest(); setManifestMenuOpen(false); }}
                  >
                    New Manifest
                  </button>
                  <button
                    className="manifest-menu-item"
                    onClick={() => { handleLoadManifest(); setManifestMenuOpen(false); }}
                  >
                    Load from File
                  </button>
                </div>
              )}
            </div>
          </div>
        </div>
        {manifestsExpanded && (
        <div className="manifest-list">
          {savedManifests.length > 0 ? (
            savedManifests.map((manifest, idx) => (
              <div
                key={idx}
                className={`history-item ${manifest.path === manifestPath ? 'active' : ''}`}
              >
                <span className="history-item-text" onClick={() => handleSelectManifest(manifest)}>
                  {manifest.name}
                </span>
                <button
                  className="manifest-item-menu-btn"
                  onClick={(e) => {
                    e.stopPropagation();
                    handleEditManifest(manifest.path);
                  }}
                  title="Edit manifest"
                >
                  ⋯
                </button>
              </div>
            ))
          ) : (
            <>
              <div
                className={`history-item ${manifestName === "Default (Fullintel)" ? 'active' : ''}`}
                onClick={() => {
                  setManifestPath(null);
                  setManifestName("Default (Fullintel)");
                  setManifestDescription(null);
                  setManifestInputLabel(null);
                }}
              >
                Opportunity Brief
              </div>
            </>
          )}
        </div>
        )}

        <div style={{ borderBottom: "1px solid var(--border-subtle)", margin: "6px 8px" }}></div>

        {/* NEW RESEARCH BUTTON with model selection */}
        <div className="sidebar-header">
          <button className="new-search-btn" onClick={handleReset} disabled={isRunning}>
            + New Research
          </button>
          <select
            className="sidebar-model-select"
            value={model}
            onChange={(e) => setModel(e.target.value)}
            disabled={isRunning}
          >
            <optgroup label="Anthropic">
              <option value="claude-opus-4-5-20251101">Claude Opus 4.5</option>
              <option value="claude-sonnet-4-5-20250929">Claude Sonnet 4.5</option>
            </optgroup>
            <optgroup label="OpenAI">
              <option value="gpt-5.1">GPT 5.1</option>
              <option value="gpt-5.1-thinking">GPT 5.1 Thinking</option>
              <option value="gpt-5">GPT 5</option>
            </optgroup>
            <optgroup label="DeepSeek">
              <option value="deepseek-reasoner">DeepSeek R1</option>
              <option value="deepseek-chat">DeepSeek V3</option>
            </optgroup>
            <optgroup label="Google">
              <option value="gemini-3-pro-preview">Gemini 3 Pro</option>
              <option value="gemini-2.0-flash">Gemini 2.0</option>
            </optgroup>
            {customProviders.length > 0 && (
              <optgroup label="Custom">
                {customProviders.map((provider) => (
                  <option key={provider.id} value={provider.provider_key}>
                    {provider.name}
                  </option>
                ))}
              </optgroup>
            )}
          </select>
        </div>

        {/* CURRENT SESSION - Expandable (only show when research is active or has content) */}
        {(company.trim() || report || isRunning) && (
          <>
            <div className="sidebar-section">
              <div className="section-label">Current Session</div>
            </div>

            <div className="current-session-panel">
              <div
                className="current-session-header"
                onClick={() => setCurrentSessionExpanded(!currentSessionExpanded)}
              >
                <span className="current-session-title">
                  {company.trim() || "New Research"}
                </span>
                <span className="current-session-toggle">
                  {currentSessionExpanded ? "−" : "+"}
                </span>
              </div>
              {currentSessionExpanded && (
                <div className="current-session-expanded">
                  <div className="current-session-row">
                    <span className="current-session-label">Model:</span>
                    <select
                      className="session-model-select"
                      value={model}
                      onChange={(e) => setModel(e.target.value)}
                      disabled={isRunning}
                    >
                      <optgroup label="Anthropic">
                        <option value="claude-opus-4-5-20251101">Claude Opus 4.5</option>
                        <option value="claude-sonnet-4-5-20250929">Claude Sonnet 4.5</option>
                      </optgroup>
                      <optgroup label="OpenAI">
                        <option value="gpt-5.1">GPT 5.1</option>
                        <option value="gpt-5">GPT 5</option>
                      </optgroup>
                      <optgroup label="DeepSeek">
                        <option value="deepseek-reasoner">DeepSeek R1</option>
                        <option value="deepseek-chat">DeepSeek V3</option>
                      </optgroup>
                      <optgroup label="Google">
                        <option value="gemini-3-pro-preview">Gemini 3 Pro</option>
                        <option value="gemini-2.0-flash">Gemini 2.0</option>
                      </optgroup>
                    </select>
                  </div>
                  <div className="current-session-row">
                    <span className="current-session-label">Manifest:</span>
                    <span className="current-session-value">{manifestName}</span>
                  </div>
                  {!apiKeyConfigured && (
                    <div className="current-session-row" style={{ color: "var(--status-invalid)" }}>
                      <span className="current-session-label">!</span>
                      <span className="current-session-value">No API key configured</span>
                    </div>
                  )}
                  {isRunning ? (
                    <button
                      className="refire-btn"
                      onClick={handleAbort}
                      style={{ background: "var(--status-invalid)", color: "white" }}
                    >
                      Stop
                    </button>
                  ) : (
                    <button
                      className="refire-btn"
                      onClick={startResearch}
                      disabled={!company.trim() || !apiKeyConfigured}
                    >
                      ↻ Re-run with changes
                    </button>
                  )}
                </div>
              )}
            </div>
          </>
        )}

        {/* PROJECTS SECTION - Collapsible, limited height, doesn't push history down */}
        {projects.length > 0 && (
          <div className="history-list" style={{ marginBottom: "8px", flex: "0 0 auto", maxHeight: projectsExpanded ? "35%" : "auto", overflowY: "auto" }}>
            <div className="sidebar-section">
              <div
                className="section-label"
                onClick={() => setProjectsExpanded(!projectsExpanded)}
                style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px" }}
              >
                <span style={{ fontSize: "10px", minWidth: "12px" }}>{projectsExpanded ? "▼" : "▶"}</span>
                Projects
              </div>
            </div>
            {projectsExpanded && projects.map((project) => (
              <div key={project.id} className="project-folder-wrapper">
                {/* Project folder header */}
                <div
                  className={`history-item project-folder ${expandedProjects.has(project.id) ? 'expanded' : ''}`}
                  style={{ display: "flex", justifyContent: "space-between", alignItems: "center" }}
                >
                  {renamingProject === project.id ? (
                    <input
                      type="text"
                      value={renameProjectValue}
                      onChange={(e) => setRenameProjectValue(e.target.value)}
                      onBlur={() => saveRenameProject(project.id)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") saveRenameProject(project.id);
                        if (e.key === "Escape") setRenamingProject(null);
                      }}
                      autoFocus
                      style={{
                        flex: 1,
                        padding: "1px 4px",
                        fontSize: "8px",
                        border: "1px solid var(--blue-300)",
                        borderRadius: "2px",
                        background: "#fff",
                        marginRight: "4px"
                      }}
                      onClick={(e) => e.stopPropagation()}
                    />
                  ) : (
                    <span
                      className="history-item-text"
                      onClick={() => toggleProjectExpansion(project)}
                      style={{ flex: 1, cursor: "pointer" }}
                    >
                      <span style={{ marginRight: "3px", fontSize: "6px", color: "var(--text-muted)" }}>
                        {expandedProjects.has(project.id) ? "▼" : "▶"}
                      </span>
                      {project.name}
                      <span style={{ color: "var(--text-muted)", fontSize: "7px", marginLeft: "4px" }}>
                        ({project.session_count})
                      </span>
                    </span>
                  )}
                  <div className="project-menu-wrapper" style={{ position: "relative" }}>
                    <button
                      className="session-menu-btn"
                      onClick={(e) => openProjectMenu(e, project.id)}
                      title="Project options"
                    >
                      ⋯
                    </button>
                  </div>
                </div>

                {/* Expanded project sessions (inline) */}
                {expandedProjects.has(project.id) && (
                  <div className="project-sessions-inline">
                    {(projectSessionsMap.get(project.id) || []).length === 0 ? (
                      <div className="project-session-item empty">
                        No sessions in this project
                      </div>
                    ) : (
                      (projectSessionsMap.get(project.id) || []).map((session) => (
                        <div
                          key={session.id}
                          className="project-session-item"
                          onClick={() => viewSession(session)}
                        >
                          <span className="project-session-name">{session.company}</span>
                          <button
                            className="session-menu-btn"
                            onClick={(e) => {
                              e.stopPropagation();
                              const rect = e.currentTarget.getBoundingClientRect();
                              setProjectSessionMenuPosition({ top: rect.bottom + 2, left: rect.right - 80 });
                              setProjectSessionMenuOpen({ projectId: project.id, sessionId: session.id });
                            }}
                            title="Session options"
                          >
                            ⋯
                          </button>
                        </div>
                      ))
                    )}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}

        {/* PROJECT MENU DROPDOWN (fixed position) */}
        {projectMenuOpen !== null && (
          <div
            className="session-menu-dropdown project-menu-wrapper"
            style={{
              position: "fixed",
              top: projectMenuPosition.top,
              left: projectMenuPosition.left,
              zIndex: 1000
            }}
          >
            <button
              className="session-menu-item"
              onClick={() => {
                const project = projects.find(p => p.id === projectMenuOpen);
                if (project) startRenameProject(project);
              }}
            >
              Rename
            </button>
            <button
              className="session-menu-item"
              onClick={async () => {
                const project = projects.find(p => p.id === projectMenuOpen);
                if (project) {
                  try {
                    await invoke("archive_project", { projectId: project.id });
                    await loadProjects();
                    await loadArchivedItems();
                  } catch (e) {
                    console.error("Failed to archive project:", e);
                  }
                }
                setProjectMenuOpen(null);
              }}
            >
              Archive
            </button>
            <div className="session-menu-divider" />
            <button
              className="session-menu-item danger"
              onClick={() => {
                const project = projects.find(p => p.id === projectMenuOpen);
                if (project) deleteProject(project.id, project.name);
              }}
            >
              Delete Project
            </button>
          </div>
        )}

        {/* PROJECT SESSION MENU DROPDOWN (for sessions inside projects) */}
        {projectSessionMenuOpen !== null && (
          <div
            className="session-menu-dropdown"
            style={{
              position: "fixed",
              top: projectSessionMenuPosition.top,
              left: projectSessionMenuPosition.left,
              zIndex: 1000
            }}
          >
            <button
              className="session-menu-item"
              onClick={() => {
                const session = projectSessionsMap.get(projectSessionMenuOpen.projectId)?.find(s => s.id === projectSessionMenuOpen.sessionId);
                if (session) viewSession(session);
                setProjectSessionMenuOpen(null);
              }}
            >
              Open
            </button>
            <div className="session-menu-divider" />
            <button
              className="session-menu-item danger"
              onClick={() => {
                removeSessionFromProject(projectSessionMenuOpen.projectId, projectSessionMenuOpen.sessionId);
                setProjectSessionMenuOpen(null);
              }}
            >
              Remove from Project
            </button>
          </div>
        )}

        {/* ARCHIVED SECTION - limited height, doesn't push history down */}
        {(archivedProjects.length > 0 || archivedSessions.length > 0) && (
          <div className="history-list" style={{ marginBottom: "8px", flex: "0 0 auto", maxHeight: "25%", overflowY: "auto" }}>
            <div className="sidebar-section">
              <div
                className="section-label"
                onClick={() => setShowArchived(!showArchived)}
                style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px" }}
              >
                <span style={{ fontSize: "10px", minWidth: "12px" }}>{showArchived ? "▼" : "▶"}</span>
                Archived ({archivedProjects.length + archivedSessions.length})
              </div>
            </div>
            {showArchived && (
              <>
                {/* Archived Projects */}
                {archivedProjects.map((project) => (
                  <div key={`archived-project-${project.id}`} className="history-item" style={{ opacity: 0.7 }}>
                    <span className="history-item-text" style={{ fontStyle: "italic" }}>
                      📁 {project.name}
                    </span>
                    <div className="session-menu-wrapper">
                      <button
                        className="session-menu-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (archivedMenuOpen?.type === 'project' && archivedMenuOpen?.id === project.id) {
                            setArchivedMenuOpen(null);
                          } else {
                            const rect = e.currentTarget.getBoundingClientRect();
                            setArchivedMenuPosition({ top: rect.bottom + 2, left: rect.right - 140 });
                            setArchivedMenuOpen({ type: 'project', id: project.id });
                          }
                        }}
                        title="Archived project actions"
                      >
                        ⋯
                      </button>
                      {archivedMenuOpen?.type === 'project' && archivedMenuOpen?.id === project.id && (
                        <div className="session-menu-dropdown" style={{ top: archivedMenuPosition.top, left: archivedMenuPosition.left }}>
                          <button
                            className="session-menu-item"
                            onClick={async (e) => {
                              e.stopPropagation();
                              try {
                                await invoke("unarchive_project", { projectId: project.id });
                                await loadProjects();
                                await loadArchivedItems();
                              } catch (err) {
                                console.error("Failed to unarchive project:", err);
                              }
                              setArchivedMenuOpen(null);
                            }}
                          >
                            Unarchive
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item danger"
                            onClick={() => {
                              deleteProject(project.id, project.name);
                              setArchivedMenuOpen(null);
                            }}
                          >
                            Delete
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
                {/* Archived Sessions */}
                {archivedSessions.map((session) => (
                  <div key={`archived-session-${session.id}`} className="history-item" style={{ opacity: 0.7 }}>
                    <span
                      className="history-item-text"
                      style={{ fontStyle: "italic" }}
                      onClick={() => viewSession(session)}
                    >
                      {session.company}
                    </span>
                    <div className="session-menu-wrapper">
                      <button
                        className="session-menu-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (archivedMenuOpen?.type === 'session' && archivedMenuOpen?.id === session.id) {
                            setArchivedMenuOpen(null);
                          } else {
                            const rect = e.currentTarget.getBoundingClientRect();
                            setArchivedMenuPosition({ top: rect.bottom + 2, left: rect.right - 140 });
                            setArchivedMenuOpen({ type: 'session', id: session.id });
                          }
                        }}
                        title="Archived session actions"
                      >
                        ⋯
                      </button>
                      {archivedMenuOpen?.type === 'session' && archivedMenuOpen?.id === session.id && (
                        <div className="session-menu-dropdown" style={{ top: archivedMenuPosition.top, left: archivedMenuPosition.left }}>
                          <button
                            className="session-menu-item"
                            onClick={async (e) => {
                              e.stopPropagation();
                              try {
                                await invoke("unarchive_session", { sessionId: session.id });
                                await refreshSessionsList();
                                await loadArchivedItems();
                              } catch (err) {
                                console.error("Failed to unarchive session:", err);
                              }
                              setArchivedMenuOpen(null);
                            }}
                          >
                            Unarchive
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item danger"
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteSession(session.id);
                              setArchivedMenuOpen(null);
                            }}
                          >
                            Delete
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
              </>
            )}
          </div>
        )}

        {/* SESSION HISTORY - Collapsible with sub-sections */}
        <div className="history-list">
          <div className="sidebar-section">
            <div
              className="section-label"
              onClick={() => setSessionHistoryExpanded(!sessionHistoryExpanded)}
              style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px" }}
            >
              <span style={{ fontSize: "10px", minWidth: "12px" }}>{sessionHistoryExpanded ? "▼" : "▶"}</span>
              Session History
            </div>
          </div>

          {sessionHistoryExpanded && (
          <>
          {/* Today's sessions - Collapsible sub-section */}
          {researchSessions.filter(s => {
            const today = new Date();
            const sessionDate = new Date(s.created_at);
            return sessionDate.toDateString() === today.toDateString();
          }).length > 0 && (
            <>
              <div className="sidebar-section">
                <div
                  className="section-label sub-section"
                  onClick={() => setTodayExpanded(!todayExpanded)}
                  style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px", paddingLeft: "8px" }}
                >
                  <span style={{ fontSize: "8px", minWidth: "10px" }}>{todayExpanded ? "▼" : "▶"}</span>
                  Today
                </div>
              </div>
              {todayExpanded && researchSessions
                .filter(s => {
                  const today = new Date();
                  const sessionDate = new Date(s.created_at);
                  return sessionDate.toDateString() === today.toDateString();
                })
                .map((session) => (
                  <div
                    key={session.id}
                    className={`history-item ${selectedSession?.id === session.id ? 'active' : ''}`}
                  >
                    <span className="history-item-text" onClick={() => viewSession(session)}>
                      {session.company}
                    </span>
                    <div className="session-menu-wrapper">
                      <button
                        className="session-menu-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (sessionMenuOpen === session.id) {
                            setSessionMenuOpen(null);
                          } else {
                            const rect = e.currentTarget.getBoundingClientRect();
                            setMenuPosition({ top: rect.bottom + 2, left: rect.right - 140 });
                            setSessionMenuOpen(session.id);
                          }
                        }}
                        title="Session actions"
                      >
                        ⋯
                      </button>
                      {sessionMenuOpen === session.id && (
                        <div className="session-menu-dropdown" style={{ top: menuPosition.top, left: menuPosition.left }}>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              setShowProjectModal(true);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Create Project
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              setAddToProjectSession(session);
                              setShowAddToProjectDropdown(true);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Add to Project
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              viewSession(session);
                              setSessionMenuOpen(null);
                            }}
                          >
                            View
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              resumeSession(session);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Resume
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "save");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Save As...
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "copy");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Copy
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "print");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Print
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              exportSession(session.id, session.company);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Export
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={async (e) => {
                              e.stopPropagation();
                              try {
                                await invoke("archive_session", { sessionId: session.id });
                                await refreshSessionsList();
                                await loadArchivedItems();
                              } catch (err) {
                                console.error("Failed to archive session:", err);
                              }
                              setSessionMenuOpen(null);
                            }}
                          >
                            Archive
                          </button>
                          <button
                            className="session-menu-item danger"
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteSession(session.id);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Delete
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
            </>
          )}

          {/* Yesterday - Collapsible sub-section */}
          {researchSessions.filter(s => {
            const yesterday = new Date();
            yesterday.setDate(yesterday.getDate() - 1);
            const sessionDate = new Date(s.created_at);
            return sessionDate.toDateString() === yesterday.toDateString();
          }).length > 0 && (
            <>
              <div className="sidebar-section">
                <div
                  className="section-label sub-section"
                  onClick={() => setYesterdayExpanded(!yesterdayExpanded)}
                  style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px", paddingLeft: "8px" }}
                >
                  <span style={{ fontSize: "8px", minWidth: "10px" }}>{yesterdayExpanded ? "▼" : "▶"}</span>
                  Yesterday
                </div>
              </div>
              {yesterdayExpanded && researchSessions
                .filter(s => {
                  const yesterday = new Date();
                  yesterday.setDate(yesterday.getDate() - 1);
                  const sessionDate = new Date(s.created_at);
                  return sessionDate.toDateString() === yesterday.toDateString();
                })
                .map((session) => (
                  <div
                    key={session.id}
                    className={`history-item ${selectedSession?.id === session.id ? 'active' : ''}`}
                  >
                    <span className="history-item-text" onClick={() => viewSession(session)}>
                      {session.company}
                    </span>
                    <div className="session-menu-wrapper">
                      <button
                        className="session-menu-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (sessionMenuOpen === session.id) {
                            setSessionMenuOpen(null);
                          } else {
                            const rect = e.currentTarget.getBoundingClientRect();
                            setMenuPosition({ top: rect.bottom + 2, left: rect.right - 140 });
                            setSessionMenuOpen(session.id);
                          }
                        }}
                        title="Session actions"
                      >
                        ⋯
                      </button>
                      {sessionMenuOpen === session.id && (
                        <div className="session-menu-dropdown" style={{ top: menuPosition.top, left: menuPosition.left }}>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              setShowProjectModal(true);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Create Project
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              setAddToProjectSession(session);
                              setShowAddToProjectDropdown(true);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Add to Project
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              viewSession(session);
                              setSessionMenuOpen(null);
                            }}
                          >
                            View
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              resumeSession(session);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Resume
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "save");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Save As...
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "copy");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Copy
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "print");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Print
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              exportSession(session.id, session.company);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Export
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={async (e) => {
                              e.stopPropagation();
                              try {
                                await invoke("archive_session", { sessionId: session.id });
                                await refreshSessionsList();
                                await loadArchivedItems();
                              } catch (err) {
                                console.error("Failed to archive session:", err);
                              }
                              setSessionMenuOpen(null);
                            }}
                          >
                            Archive
                          </button>
                          <button
                            className="session-menu-item danger"
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteSession(session.id);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Delete
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
            </>
          )}

          {/* Previous 7 Days - Collapsible sub-section */}
          {researchSessions.filter(s => {
            const today = new Date();
            const yesterday = new Date();
            yesterday.setDate(yesterday.getDate() - 1);
            const weekAgo = new Date();
            weekAgo.setDate(weekAgo.getDate() - 7);
            const sessionDate = new Date(s.created_at);
            return sessionDate < yesterday && sessionDate >= weekAgo;
          }).length > 0 && (
            <>
              <div className="sidebar-section">
                <div
                  className="section-label sub-section"
                  onClick={() => setPrevious7DaysExpanded(!previous7DaysExpanded)}
                  style={{ cursor: "pointer", display: "flex", alignItems: "center", gap: "4px", paddingLeft: "8px" }}
                >
                  <span style={{ fontSize: "8px", minWidth: "10px" }}>{previous7DaysExpanded ? "▼" : "▶"}</span>
                  Previous 7 Days
                </div>
              </div>
              {previous7DaysExpanded && researchSessions
                .filter(s => {
                  const today = new Date();
                  const yesterday = new Date();
                  yesterday.setDate(yesterday.getDate() - 1);
                  const weekAgo = new Date();
                  weekAgo.setDate(weekAgo.getDate() - 7);
                  const sessionDate = new Date(s.created_at);
                  return sessionDate < yesterday && sessionDate >= weekAgo;
                })
                .map((session) => (
                  <div
                    key={session.id}
                    className={`history-item ${selectedSession?.id === session.id ? 'active' : ''}`}
                  >
                    <span className="history-item-text" onClick={() => viewSession(session)}>
                      {session.company}
                    </span>
                    <div className="session-menu-wrapper">
                      <button
                        className="session-menu-btn"
                        onClick={(e) => {
                          e.stopPropagation();
                          if (sessionMenuOpen === session.id) {
                            setSessionMenuOpen(null);
                          } else {
                            const rect = e.currentTarget.getBoundingClientRect();
                            setMenuPosition({ top: rect.bottom + 2, left: rect.right - 140 });
                            setSessionMenuOpen(session.id);
                          }
                        }}
                        title="Session actions"
                      >
                        ⋯
                      </button>
                      {sessionMenuOpen === session.id && (
                        <div className="session-menu-dropdown" style={{ top: menuPosition.top, left: menuPosition.left }}>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              setShowProjectModal(true);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Create Project
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              setAddToProjectSession(session);
                              setShowAddToProjectDropdown(true);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Add to Project
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              viewSession(session);
                              setSessionMenuOpen(null);
                            }}
                          >
                            View
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              resumeSession(session);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Resume
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "save");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Save As...
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "copy");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Copy
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              sessionFileAction(session, "print");
                              setSessionMenuOpen(null);
                            }}
                          >
                            Print
                          </button>
                          <button
                            className="session-menu-item"
                            onClick={(e) => {
                              e.stopPropagation();
                              exportSession(session.id, session.company);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Export
                          </button>
                          <div className="session-menu-divider"></div>
                          <button
                            className="session-menu-item"
                            onClick={async (e) => {
                              e.stopPropagation();
                              try {
                                await invoke("archive_session", { sessionId: session.id });
                                await refreshSessionsList();
                                await loadArchivedItems();
                              } catch (err) {
                                console.error("Failed to archive session:", err);
                              }
                              setSessionMenuOpen(null);
                            }}
                          >
                            Archive
                          </button>
                          <button
                            className="session-menu-item danger"
                            onClick={(e) => {
                              e.stopPropagation();
                              deleteSession(session.id);
                              setSessionMenuOpen(null);
                            }}
                          >
                            Delete
                          </button>
                        </div>
                      )}
                    </div>
                  </div>
                ))}
            </>
          )}
          </>
          )}

          {researchSessions.length === 0 && (
            <div className="history-item" style={{ color: "var(--text-faint)", cursor: "default" }}>
              No research sessions yet
            </div>
          )}
        </div>

        {/* SIDEBAR FOOTER - Settings & User */}
        <div className="sidebar-footer">
          <button className="settings-btn" onClick={() => setShowSettings(true)}>
            Settings
          </button>
          <div className="user-row">
            <div className="user-avatar">
              {currentUser.username.charAt(0).toUpperCase()}
            </div>
            <span className="user-name">{currentUser.username}</span>
            <span className="logout-btn" onClick={handleLogout}>logout</span>
          </div>
        </div>
      </div>

      {/* MAIN CHAT AREA */}
      <div className="main">
        <div className="main-header">
          <span className="header-title">
            {selectedSession
              ? `${selectedSession.company} - ${selectedSession.manifest_name || "Session"}`
              : company.trim()
                ? `${company} - ${manifestName}`
                : "FullIntel Agent"}
          </span>
          <img src={fullintelLogoWide} alt="FullIntel" className="header-logo-wide" />
        </div>

        <div className="chat-area" ref={splitContainerRef}>
          {/* Main Content Display - PRIMARY real estate for LLM output */}
          {selectedSession ? (
            /* Session Detail View */
            <div className="content-display" style={conversation.length > 0 ? {
              maxHeight: `${briefPaneHeight}%`,
              flex: `0 0 ${briefPaneHeight}%`,
              minHeight: "100px"
            } : undefined}>
              <div className="content-display-header">
                <span className="content-display-title">{selectedSession.company}</span>
                <div className="header-phase-progress">
                  {sessionPhaseOutputs.map((output, idx) => (
                    <span
                      key={output.phase_id}
                      className={`header-phase ${output.status === 'completed' ? 'complete' : output.status === 'running' ? 'active' : ''}`}
                    >
                      <span className="header-phase-dot"></span>
                      {getPhaseDisplayName(output.phase_name)}
                    </span>
                  ))}
                </div>
              </div>
              <div className="content-display-body">
                {/* Session Actions Bar */}
                <div style={{ display: "flex", gap: "12px", alignItems: "center", marginBottom: "12px" }}>
                  {selectedSession.status === "in_progress" && (
                    <button
                      className="refire-btn"
                      onClick={() => resumeSession(selectedSession)}
                      disabled={isResuming}
                    >
                      {isResuming ? "Resuming..." : "Resume"}
                    </button>
                  )}
                  <div style={{ display: "flex", gap: "8px", marginLeft: "auto" }}>
                    <button className="text-btn" onClick={() => handleCopy()}>
                      copy {copyStatus && <span style={{ color: "var(--status-valid)" }}>{copyStatus}</span>}
                    </button>
                    <button className="text-btn" onClick={() => handlePrint()}>
                      print
                    </button>
                    <button className="text-btn" onClick={() => handleSave()}>
                      save
                    </button>
                    <span style={{ borderLeft: "1px solid var(--border-subtle)", margin: "0 4px" }}></span>
                    <button className="text-btn" onClick={closeSessionView}>
                      close
                    </button>
                  </div>
                </div>

                {/* Phase Outputs */}
                {sessionPhaseOutputs.map((output) => (
                  <div key={output.id} style={{ marginTop: "16px", paddingTop: "12px", borderTop: "1px solid var(--border-subtle)" }}>
                    <div style={{ display: "flex", alignItems: "center", gap: "8px", marginBottom: "8px" }}>
                      <span style={{ fontWeight: 400, color: "var(--text-secondary)" }}>
                        {output.phase_name}
                      </span>
                      {(output.system_prompt || output.user_input) && (
                        <button
                          className="text-btn"
                          onClick={() => togglePromptExpanded(output.phase_id)}
                          style={{ fontSize: "8px" }}
                        >
                          {expandedPrompts.has(output.phase_id) ? "hide prompts" : "view prompts"}
                        </button>
                      )}
                    </div>

                    {expandedPrompts.has(output.phase_id) && (output.system_prompt || output.user_input) && (
                      <div style={{ background: "var(--blue-50)", padding: "12px", borderRadius: "4px", marginBottom: "8px", fontSize: "9px" }}>
                        {/* Prompt editing mode */}
                        {editingPrompt?.phaseId === output.phase_id ? (
                          <>
                            <div style={{ marginBottom: "12px" }}>
                              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "4px" }}>
                                <strong>System Prompt:</strong>
                              </div>
                              <textarea
                                value={editingPrompt.systemPrompt}
                                onChange={(e) => setEditingPrompt({ ...editingPrompt, systemPrompt: e.target.value })}
                                style={{
                                  width: "100%",
                                  minHeight: "120px",
                                  padding: "8px",
                                  fontSize: "9px",
                                  fontFamily: "inherit",
                                  border: "1px solid var(--blue-300)",
                                  borderRadius: "3px",
                                  resize: "vertical",
                                  background: "var(--bg-card)",
                                }}
                              />
                            </div>
                            <div style={{ marginBottom: "12px" }}>
                              <div style={{ display: "flex", justifyContent: "space-between", alignItems: "center", marginBottom: "4px" }}>
                                <strong>User Input:</strong>
                              </div>
                              <textarea
                                value={editingPrompt.userInput}
                                onChange={(e) => setEditingPrompt({ ...editingPrompt, userInput: e.target.value })}
                                style={{
                                  width: "100%",
                                  minHeight: "80px",
                                  padding: "8px",
                                  fontSize: "9px",
                                  fontFamily: "inherit",
                                  border: "1px solid var(--blue-300)",
                                  borderRadius: "3px",
                                  resize: "vertical",
                                  background: "var(--bg-card)",
                                }}
                              />
                            </div>
                            <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end" }}>
                              <button
                                className="text-btn"
                                onClick={() => setEditingPrompt(null)}
                                style={{ fontSize: "8px" }}
                              >
                                cancel
                              </button>
                              <button
                                className="refire-btn"
                                onClick={() => relaunchPhase(output)}
                                disabled={relaunchingPhase === output.phase_id}
                                style={{ fontSize: "9px", padding: "4px 12px" }}
                              >
                                {relaunchingPhase === output.phase_id ? "Running..." : "Relaunch Phase"}
                              </button>
                            </div>
                          </>
                        ) : (
                          <>
                            {output.system_prompt && (
                              <div style={{ marginBottom: "8px" }}>
                                <strong>System Prompt:</strong>
                                <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0", background: "var(--bg-card)", padding: "6px", borderRadius: "2px" }}>{output.system_prompt}</pre>
                              </div>
                            )}
                            {output.user_input && (
                              <div style={{ marginBottom: "8px" }}>
                                <strong>User Input:</strong>
                                <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0", background: "var(--bg-card)", padding: "6px", borderRadius: "2px" }}>{output.user_input}</pre>
                              </div>
                            )}
                            {/* Edit and Relaunch buttons */}
                            <div style={{ display: "flex", gap: "8px", justifyContent: "flex-end", marginTop: "8px", paddingTop: "8px", borderTop: "1px solid var(--border-subtle)" }}>
                              <button
                                className="text-btn"
                                onClick={() => startEditingPrompt(output)}
                                style={{ fontSize: "8px" }}
                              >
                                edit prompts
                              </button>
                              <button
                                className="refire-btn"
                                onClick={() => relaunchPhase(output)}
                                disabled={relaunchingPhase === output.phase_id || (!output.system_prompt && !output.user_input)}
                                style={{ fontSize: "9px", padding: "4px 12px" }}
                              >
                                {relaunchingPhase === output.phase_id ? "Running..." : "Relaunch"}
                              </button>
                            </div>
                          </>
                        )}
                      </div>
                    )}

                    {output.output && (
                      <div style={{ fontSize: "10px", lineHeight: 1.6, whiteSpace: "pre-wrap" }}>
                        {output.output}
                      </div>
                    )}
                    {output.error && (
                      <div style={{ color: "var(--status-invalid)", fontSize: "10px" }}>
                        Error: {output.error}
                      </div>
                    )}
                  </div>
                ))}
              </div>
            </div>
          ) : report ? (
            /* Report Display - Main content with phase progress */
            <div className="content-display">
              <div className="content-display-header">
                <span className="content-display-title">Research Output{company ? ` [${company}]` : ""}</span>
                <div className="header-phase-progress">
                  {phases.map((phase) => (
                    <span
                      key={phase.id}
                      className={`header-phase clickable ${phase.status === 'completed' ? 'complete' : phase.status === 'running' ? 'active' : ''}`}
                      onClick={(e) => handlePhaseClick(e, phase.id)}
                      title={`Click for options - ${phase.name}`}
                    >
                      <span className="header-phase-dot"></span>
                      {getPhaseDisplayName(phase.name)}
                      {/* Phase dropdown menu */}
                      {phaseMenuOpen === phase.id && (
                        <div
                          className="phase-menu-dropdown"
                          style={{ position: 'fixed', top: phaseMenuPosition.top, left: phaseMenuPosition.left }}
                          onClick={(e) => e.stopPropagation()}
                        >
                          <button
                            className="phase-menu-item"
                            onClick={() => scrollToPhase(phase.name)}
                          >
                            📍 Jump to Section
                          </button>
                          <button
                            className="phase-menu-item"
                            onClick={() => resumeFromPhase(phase.id)}
                            disabled={isRunning}
                          >
                            🔄 Resume from Here
                          </button>
                        </div>
                      )}
                    </span>
                  ))}
                </div>
              </div>
              <div className="content-display-body" ref={contentBodyRef}>
                {renderReportWithTOC(report)}
              </div>
            </div>
          ) : (
            /* Welcome / Empty State / Active Research */
            <div className="content-display" style={conversation.length > 0 ? {
              maxHeight: `${briefPaneHeight}%`,
              flex: `0 0 ${briefPaneHeight}%`,
              minHeight: "100px"
            } : undefined}>
              <div className="content-display-header">
                <div className="content-display-title-section">
                  <span className="content-display-title">
                    {isRunning ? "Research in Progress" : manifestName}
                  </span>
                  {/* Show manifest metadata (id, version, description) below title when not running */}
                  {!isRunning && (manifestId || manifestVersion || manifestDescription) && (
                    <div className="manifest-metadata">
                      {(manifestId || manifestVersion) && (
                        <div className="manifest-id-version">
                          {manifestId && <span className="manifest-id">{manifestId}</span>}
                          {manifestId && manifestVersion && <span className="manifest-separator">•</span>}
                          {manifestVersion && <span className="manifest-version">v{manifestVersion}</span>}
                        </div>
                      )}
                      {manifestDescription && (
                        <div className="manifest-description">{manifestDescription}</div>
                      )}
                    </div>
                  )}
                </div>
                {(isRunning || logs.length > 0) && (
                  <div className="header-phase-progress">
                    {phases.map((phase) => (
                      <span
                        key={phase.id}
                        className={`header-phase clickable ${phase.status === 'completed' ? 'complete' : phase.status === 'running' ? 'active' : ''}`}
                        onClick={(e) => handlePhaseClick(e, phase.id)}
                        title={`Click for options - ${phase.name}`}
                      >
                        <span className="header-phase-dot"></span>
                        {getPhaseDisplayName(phase.name)}
                        {/* Phase dropdown menu */}
                        {phaseMenuOpen === phase.id && (
                          <div
                            className="phase-menu-dropdown"
                            style={{ position: 'fixed', top: phaseMenuPosition.top, left: phaseMenuPosition.left }}
                            onClick={(e) => e.stopPropagation()}
                          >
                            <button
                              className="phase-menu-item"
                              onClick={() => resumeFromPhase(phase.id)}
                              disabled={isRunning}
                            >
                              🔄 Resume from Here
                            </button>
                          </div>
                        )}
                      </span>
                    ))}
                  </div>
                )}
              </div>
              <div className="content-display-body">
                {logs.length === 0 && !isRunning ? (
                  <div style={{ textAlign: "center", padding: "48px 24px" }}>
                    <h2 style={{ fontSize: "18px", fontWeight: 300, marginBottom: "12px", color: "var(--text-primary)" }}>
                      Welcome{currentUser ? `, ${currentUser.first_name || currentUser.username}` : ""}
                    </h2>
                    {manifestDescription && (
                      <p style={{ color: "var(--text-secondary)", marginBottom: "16px", fontStyle: "italic", maxWidth: "500px", margin: "0 auto 16px" }}>
                        {manifestDescription}
                      </p>
                    )}
                    <p style={{ color: "var(--text-muted)", marginBottom: "24px" }}>
                      {manifestInputLabel || "Enter research subject for manifest processing."}
                    </p>
                    {!apiKeyConfigured && (
                      <button
                        className="refire-btn"
                        onClick={() => setShowSettings(true)}
                      >
                        Configure API Keys
                      </button>
                    )}
                    {/* Research subject input (manifest-specific label) */}
                    <div style={{ maxWidth: "300px", margin: "24px auto 0" }}>
                      <input
                        type="text"
                        className="chat-input"
                        value={company}
                        onChange={(e) => setCompany(e.target.value)}
                        placeholder={manifestInputLabel || "Enter research subject..."}
                        disabled={isRunning}
                        onKeyDown={(e) => e.key === "Enter" && !isRunning && apiKeyConfigured && company.trim() && startResearch()}
                        style={{ marginBottom: "8px" }}
                      />
                      <button
                        className="refire-btn"
                        onClick={startResearch}
                        disabled={isRunning || !company.trim() || !apiKeyConfigured}
                        style={{ width: "100%" }}
                      >
                        ▶ Generate Brief
                      </button>
                    </div>
                  </div>
                ) : (
                  <div>
                    {/* IM-5045: Live Prompt View during active research */}
                    {livePhasePrompt && (
                      <div style={{ marginBottom: "16px", border: "1px solid var(--blue-200)", borderRadius: "4px", overflow: "hidden" }}>
                        <div
                          style={{
                            display: "flex",
                            alignItems: "center",
                            justifyContent: "space-between",
                            padding: "8px 12px",
                            background: "var(--blue-100)",
                            cursor: "pointer",
                          }}
                          onClick={() => setShowLivePrompt(!showLivePrompt)}
                        >
                          <span style={{ fontSize: "10px", fontWeight: 500, color: "var(--blue-700)" }}>
                            {showLivePrompt ? "▼" : "▶"} Phase: {livePhasePrompt.phaseName}
                          </span>
                          <span style={{ fontSize: "8px", color: "var(--blue-600)" }}>
                            {showLivePrompt ? "Hide Prompts" : "View Prompts"}
                          </span>
                        </div>
                        {showLivePrompt && (
                          <div style={{ padding: "12px", background: "var(--blue-50)", fontSize: "9px" }}>
                            {livePhasePrompt.systemPrompt && (
                              <div style={{ marginBottom: "8px" }}>
                                <strong style={{ color: "var(--text-secondary)" }}>System Prompt:</strong>
                                <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0", background: "var(--bg-card)", padding: "6px", borderRadius: "2px", fontSize: "8px", maxHeight: "150px", overflow: "auto" }}>
                                  {livePhasePrompt.systemPrompt}
                                </pre>
                              </div>
                            )}
                            {livePhasePrompt.userInput && (
                              <div>
                                <strong style={{ color: "var(--text-secondary)" }}>User Input:</strong>
                                <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0", background: "var(--bg-card)", padding: "6px", borderRadius: "2px", fontSize: "8px", maxHeight: "150px", overflow: "auto" }}>
                                  {livePhasePrompt.userInput}
                                </pre>
                              </div>
                            )}
                          </div>
                        )}
                      </div>
                    )}
                    {/* Live Response - stays at TOP of display */}
                    {streamingOutput && (
                      <div style={{ padding: "12px", background: "var(--blue-50)", borderRadius: "3px", marginBottom: "16px" }}>
                        <div style={{ fontSize: "9px", color: "var(--blue-600)", marginBottom: "6px", fontWeight: 400 }}>Live Response</div>
                        <pre style={{ fontSize: "11px", whiteSpace: "pre-wrap", margin: 0, lineHeight: 1.5, color: "var(--text-primary)" }}>{streamingOutput}</pre>
                      </div>
                    )}
                    {/* Token/Phase logs - BELOW response, smaller text */}
                    <div style={{ borderTop: streamingOutput ? "1px solid var(--border-subtle)" : "none", paddingTop: streamingOutput ? "12px" : "0" }}>
                      {logs.length > 0 && (
                        <div style={{ fontSize: "8px", color: "var(--text-muted)", marginBottom: "6px", fontWeight: 400 }}>Activity Log</div>
                      )}
                      {logs.map((log, i) => (
                        <div
                          key={i}
                          style={{
                            padding: "2px 0",
                            fontSize: "9px",
                            color: log.includes("ERROR") || log.includes("❌") ? "var(--status-invalid)" :
                              log.includes("✓") || log.includes("✅") ? "var(--status-valid)" :
                              "var(--text-muted)"
                          }}
                        >
                          {log}
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            </div>
          )}

          {/* Resize handle - only visible when conversation exists */}
          {conversation.length > 0 && (
            <div
              className="resize-handle"
              onMouseDown={handleResizeStart}
              style={{
                height: "16px",
                cursor: "row-resize",
                marginTop: "4px",
                marginBottom: "4px",
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                maxWidth: "720px",
                margin: "4px auto",
                userSelect: "none"
              }}
              title="Drag to resize panes"
            >
              {/* Grip handle with dots - more intuitive than a bar */}
              <div style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "center",
                gap: "3px",
                padding: "4px 12px",
                borderRadius: "4px",
                background: isResizing ? "var(--blue-100)" : "transparent",
                transition: "background 0.15s ease"
              }}>
                <span style={{
                  width: "4px",
                  height: "4px",
                  borderRadius: "50%",
                  background: isResizing ? "var(--blue-500)" : "var(--text-faint)"
                }} />
                <span style={{
                  width: "4px",
                  height: "4px",
                  borderRadius: "50%",
                  background: isResizing ? "var(--blue-500)" : "var(--text-faint)"
                }} />
                <span style={{
                  width: "4px",
                  height: "4px",
                  borderRadius: "50%",
                  background: isResizing ? "var(--blue-500)" : "var(--text-faint)"
                }} />
              </div>
            </div>
          )}

          {/* Conversation history */}
          {conversation.length > 0 && (
            <div
              className="content-display"
              style={{
                marginTop: "8px",
                minHeight: "150px",
                maxHeight: `${100 - briefPaneHeight}%`,
                flex: `0 0 ${100 - briefPaneHeight}%`
              }}
            >
              <div className="content-display-header">
                <span className="content-display-title">Conversation</span>
                <div style={{ display: "flex", gap: "8px", marginLeft: "auto", alignItems: "center" }}>
                  {conversationCopyStatus && (
                    <span style={{ fontSize: "9px", color: "var(--status-valid)" }}>{conversationCopyStatus}</span>
                  )}
                  <button className="text-btn" onClick={handleCopyConversation} title="Copy conversation to clipboard">
                    copy
                  </button>
                  <button className="text-btn" onClick={handleSaveConversation} title="Save conversation to file">
                    save
                  </button>
                  <button
                    className="text-btn"
                    onClick={handleSaveAsManifest}
                    title="Extract and save YAML manifest from conversation"
                    style={{ color: "var(--blue-600)" }}
                  >
                    ⋮ save as manifest
                  </button>
                </div>
              </div>
              <div className="content-display-body">
                {conversation.map((msg, idx) => (
                  <div key={idx} style={{ marginBottom: "12px" }}>
                    <strong style={{ fontSize: "9px", color: msg.role === "user" ? "var(--blue-600)" : "var(--text-secondary)" }}>
                      {msg.role === "user" ? "You:" : "Assistant:"}
                    </strong>
                    <pre style={{ whiteSpace: "pre-wrap", margin: "4px 0 0 0", fontFamily: "inherit" }}>{msg.content}</pre>
                  </div>
                ))}
                {isFollowupRunning && (
                  <div style={{ color: "var(--text-muted)", fontSize: "10px" }}>Thinking...</div>
                )}
              </div>
            </div>
          )}
        </div>

        {/* Chat input - always visible at bottom */}
        <div className="chat-input-wrapper">
          {/* Mode toggle: Chat vs Research */}
          <div className="mode-toggle" style={{
            display: "flex",
            gap: "4px",
            marginBottom: "6px",
            justifyContent: "center"
          }}>
            <button
              className={`mode-btn ${chatMode ? "active" : ""}`}
              onClick={() => { setChatMode(true); setModeExplicitlySet(true); }}
              style={{
                padding: "4px 12px",
                fontSize: "10px",
                border: chatMode ? "1px solid var(--blue-400)" : "1px solid var(--border-color)",
                borderRadius: "4px",
                background: chatMode ? "var(--blue-100)" : "transparent",
                color: chatMode ? "var(--blue-700)" : "var(--text-secondary)",
                cursor: "pointer",
                fontWeight: chatMode ? 600 : 400
              }}
              title="Chat freely without running research"
            >
              Chat
            </button>
            <button
              className={`mode-btn ${!chatMode ? "active" : ""}`}
              onClick={() => { setChatMode(false); setModeExplicitlySet(true); }}
              style={{
                padding: "4px 12px",
                fontSize: "10px",
                border: !chatMode ? "1px solid var(--blue-400)" : "1px solid var(--border-color)",
                borderRadius: "4px",
                background: !chatMode ? "var(--blue-100)" : "transparent",
                color: !chatMode ? "var(--blue-700)" : "var(--text-secondary)",
                cursor: "pointer",
                fontWeight: !chatMode ? 600 : 400
              }}
              title={manifestInputLabel || "Enter research subject to run manifest"}
            >
              Research
            </button>
          </div>
          <div className="chat-input-container">
            <textarea
              ref={chatTextareaRef}
              className="chat-input"
              value={followupInput}
              onChange={(e) => {
                setFollowupInput(e.target.value);
                adjustTextareaHeight();
              }}
              placeholder={chatMode
                ? "Ask anything, create manifests, or explore ideas..."
                : (manifestInputLabel || "Enter research subject...")}
              disabled={isFollowupRunning || isRunning}
              rows={1}
              onKeyDown={(e) => {
                // Enter submits, Shift+Enter adds newline
                if (e.key === "Enter" && !e.shiftKey && !isFollowupRunning && !isRunning && followupInput.trim()) {
                  e.preventDefault(); // Prevent newline
                  if (!apiKeyConfigured) {
                    setShowSettings(true);
                    return;
                  }

                  if (chatMode || report || conversation.length > 0) {
                    // Chat mode OR have existing context - do chat/followup
                    handleFollowup();
                    // Reset textarea height after submit
                    setTimeout(() => {
                      if (chatTextareaRef.current) {
                        chatTextareaRef.current.style.height = "auto";
                      }
                    }, 10);
                  } else {
                    // Research mode with no context - start research
                    setCompany(followupInput.trim());
                    setFollowupInput("");
                    setTimeout(() => startResearch(), 50);
                  }
                }
              }}
            />
            <button
              className="send-btn"
              onClick={() => {
                if (!apiKeyConfigured) {
                  setShowSettings(true);
                  return;
                }
                if (!followupInput.trim()) return;

                if (chatMode || report || conversation.length > 0) {
                  // Chat mode OR have existing context - do chat/followup
                  handleFollowup();
                } else {
                  // Research mode with no context - start research
                  setCompany(followupInput.trim());
                  setFollowupInput("");
                  setTimeout(() => startResearch(), 50);
                }
              }}
              disabled={isFollowupRunning || isRunning || !followupInput.trim()}
            >
              ➤
            </button>
          </div>

          {/* PROJECT CREATION MODAL */}
          {showProjectModal && (
            <div className="modal-overlay" onClick={() => setShowProjectModal(false)}>
              <div className="modal-content" onClick={e => e.stopPropagation()} style={{ width: "420px" }}>
                <div className="modal-header">
                  <h3>Create New Project</h3>
                  <button className="modal-close" onClick={() => setShowProjectModal(false)}>×</button>
                </div>
                <div className="modal-body">
                  <div style={{ marginBottom: "16px" }}>
                    <label>Project Name *</label>
                    <input
                      type="text"
                      value={newProjectName}
                      onChange={(e) => setNewProjectName(e.target.value)}
                      placeholder="e.g., Q4 Research, Client Onboarding"
                      autoFocus
                    />
                  </div>
                  <div style={{ marginBottom: "8px" }}>
                    <label>Description (optional)</label>
                    <textarea
                      value={newProjectDescription}
                      onChange={(e) => setNewProjectDescription(e.target.value)}
                      placeholder="Brief description of this project..."
                      rows={3}
                    />
                  </div>
                  <div className="modal-actions">
                    <button
                      className="modal-btn modal-btn-secondary"
                      onClick={() => setShowProjectModal(false)}
                    >
                      Cancel
                    </button>
                    <button
                      className="modal-btn modal-btn-primary"
                      onClick={createProject}
                      disabled={!newProjectName.trim()}
                    >
                      Create Project
                    </button>
                  </div>
                </div>
              </div>
            </div>
          )}

          {/* ADD TO PROJECT DROPDOWN */}
          {showAddToProjectDropdown && addToProjectSession && (
            <div className="modal-overlay" onClick={() => { setShowAddToProjectDropdown(false); setAddToProjectSession(null); }}>
              <div className="modal-content" onClick={e => e.stopPropagation()} style={{ width: "350px" }}>
                <div className="modal-header">
                  <h3 style={{ margin: 0 }}>Add to Project</h3>
                  <button className="modal-close" onClick={() => { setShowAddToProjectDropdown(false); setAddToProjectSession(null); }}>×</button>
                </div>
                <div className="modal-body" style={{ padding: "16px" }}>
                  <p style={{ margin: "0 0 12px", color: "var(--text-secondary)", fontSize: "13px" }}>
                    Add "{addToProjectSession.company}" to a project:
                  </p>
                  {projects.length === 0 ? (
                    <div style={{ textAlign: "center", padding: "20px", color: "var(--text-secondary)" }}>
                      <p>No projects yet.</p>
                      <button
                        onClick={() => {
                          setShowAddToProjectDropdown(false);
                          setAddToProjectSession(null);
                          setShowProjectModal(true);
                        }}
                        style={{
                          padding: "8px 16px",
                          border: "none",
                          borderRadius: "6px",
                          background: "var(--accent)",
                          color: "white",
                          cursor: "pointer"
                        }}
                      >
                        Create First Project
                      </button>
                    </div>
                  ) : (
                    <div style={{ maxHeight: "200px", overflowY: "auto" }}>
                      {projects.map((project) => (
                        <div
                          key={project.id}
                          onClick={() => addSessionToProject(project.id, addToProjectSession.id)}
                          style={{
                            padding: "10px 12px",
                            marginBottom: "4px",
                            borderRadius: "6px",
                            cursor: "pointer",
                            background: "var(--surface)",
                            border: "1px solid var(--border)",
                            transition: "background 0.15s"
                          }}
                          onMouseEnter={e => e.currentTarget.style.background = "var(--surface-hover)"}
                          onMouseLeave={e => e.currentTarget.style.background = "var(--surface)"}
                        >
                          <div style={{ fontWeight: 500 }}>📁 {project.name}</div>
                          {project.description && (
                            <div style={{ fontSize: "12px", color: "var(--text-secondary)", marginTop: "2px" }}>
                              {project.description}
                            </div>
                          )}
                          <div style={{ fontSize: "11px", color: "var(--text-tertiary)", marginTop: "4px" }}>
                            {project.session_count} session{project.session_count !== 1 ? "s" : ""}
                          </div>
                        </div>
                      ))}
                    </div>
                  )}
                </div>
              </div>
            </div>
          )}

          {/* Mini color palette reference - ui-preview.html design */}
          <div className="mini-palette">
            <div className="mini-swatch" style={{ background: "var(--blue-25)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-50)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-100)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-200)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-300)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-400)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-500)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-600)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-700)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-800)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-900)" }}></div>
            <div className="mini-swatch" style={{ background: "var(--blue-950)" }}></div>
          </div>
        </div>
      </div>
    </div>
  );
}

export default App;