import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./SettingsPanel.css";

interface ApiKeyEntry {
  provider: string;
  has_key: boolean;
}

interface UserInfo {
  id: number;
  username: string;
}

interface SettingsPanelProps {
  user: UserInfo;
  onClose: () => void;
  onLogout: () => void;
}

const PROVIDER_INFO: Record<string, { name: string; placeholder: string; description: string }> = {
  anthropic: {
    name: "Anthropic (Claude)",
    placeholder: "sk-ant-api03-...",
    description: "For Claude AI models",
  },
  deepseek: {
    name: "DeepSeek",
    placeholder: "sk-...",
    description: "For DeepSeek AI models",
  },
  google: {
    name: "Google (Gemini)",
    placeholder: "AIza...",
    description: "For Google Gemini models",
  },
  openai: {
    name: "OpenAI",
    placeholder: "sk-proj-...",
    description: "For GPT models",
  },
};

export function SettingsPanel({ user, onClose, onLogout }: SettingsPanelProps) {
  const [apiKeys, setApiKeys] = useState<ApiKeyEntry[]>([]);
  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [newKeyValue, setNewKeyValue] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  useEffect(() => {
    loadApiKeys();
  }, []);

  const loadApiKeys = async () => {
    try {
      const keys = await invoke<ApiKeyEntry[]>("list_provider_keys");
      setApiKeys(keys);
    } catch (err) {
      setError(`Failed to load API keys: ${err}`);
    }
  };

  const handleSaveKey = async (provider: string) => {
    if (!newKeyValue.trim()) {
      setError("API key cannot be empty");
      return;
    }

    setIsSaving(true);
    setError(null);

    try {
      await invoke("store_provider_key", {
        provider,
        apiKey: newKeyValue.trim(),
      });
      setSuccessMessage(`${PROVIDER_INFO[provider]?.name || provider} API key saved!`);
      setEditingProvider(null);
      setNewKeyValue("");
      await loadApiKeys();

      // Clear success message after 3 seconds
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Failed to save key: ${err}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDeleteKey = async (provider: string) => {
    if (!confirm(`Delete API key for ${PROVIDER_INFO[provider]?.name || provider}?`)) {
      return;
    }

    try {
      await invoke("delete_provider_key", { provider });
      setSuccessMessage(`${PROVIDER_INFO[provider]?.name || provider} API key deleted`);
      await loadApiKeys();

      // Clear success message after 3 seconds
      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Failed to delete key: ${err}`);
    }
  };

  const handleLogout = async () => {
    try {
      await invoke("auth_logout");
      onLogout();
    } catch (err) {
      setError(`Logout failed: ${err}`);
    }
  };

  return (
    <div className="settings-overlay">
      <div className="settings-panel">
        <div className="settings-header">
          <h2>Settings</h2>
          <button className="close-btn" onClick={onClose}>
            &times;
          </button>
        </div>

        <div className="settings-content">
          {/* User Info Section */}
          <div className="settings-section">
            <h3>Account</h3>
            <div className="user-info">
              <div className="user-avatar">{user.username.charAt(0).toUpperCase()}</div>
              <div className="user-details">
                <span className="user-name">{user.username}</span>
                <button className="logout-btn" onClick={handleLogout}>
                  Sign Out
                </button>
              </div>
            </div>
          </div>

          {/* API Keys Section */}
          <div className="settings-section">
            <h3>API Keys</h3>
            <p className="section-description">
              Your API keys are encrypted and stored securely. They are never sent to our servers.
            </p>

            {error && <div className="settings-error">{error}</div>}
            {successMessage && <div className="settings-success">{successMessage}</div>}

            <div className="api-keys-list">
              {apiKeys.map((key) => {
                const info = PROVIDER_INFO[key.provider] || {
                  name: key.provider,
                  placeholder: "API Key",
                  description: "",
                };

                return (
                  <div key={key.provider} className="api-key-item">
                    <div className="api-key-header">
                      <div className="api-key-info">
                        <span className="provider-name">{info.name}</span>
                        <span className="provider-description">{info.description}</span>
                      </div>
                      <span className={`key-status ${key.has_key ? "configured" : "not-configured"}`}>
                        {key.has_key ? "Configured" : "Not Set"}
                      </span>
                    </div>

                    {editingProvider === key.provider ? (
                      <div className="api-key-edit">
                        <input
                          type="password"
                          value={newKeyValue}
                          onChange={(e) => setNewKeyValue(e.target.value)}
                          placeholder={info.placeholder}
                          disabled={isSaving}
                          autoFocus
                        />
                        <div className="api-key-actions">
                          <button
                            className="save-btn"
                            onClick={() => handleSaveKey(key.provider)}
                            disabled={isSaving}
                          >
                            {isSaving ? "Saving..." : "Save"}
                          </button>
                          <button
                            className="cancel-btn"
                            onClick={() => {
                              setEditingProvider(null);
                              setNewKeyValue("");
                              setError(null);
                            }}
                            disabled={isSaving}
                          >
                            Cancel
                          </button>
                        </div>
                      </div>
                    ) : (
                      <div className="api-key-actions">
                        <button
                          className="edit-btn"
                          onClick={() => {
                            setEditingProvider(key.provider);
                            setNewKeyValue("");
                            setError(null);
                          }}
                        >
                          {key.has_key ? "Update" : "Add Key"}
                        </button>
                        {key.has_key && (
                          <button
                            className="delete-btn"
                            onClick={() => handleDeleteKey(key.provider)}
                          >
                            Delete
                          </button>
                        )}
                      </div>
                    )}
                  </div>
                );
              })}
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
