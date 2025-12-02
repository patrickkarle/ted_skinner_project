import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./SettingsPanel.css";

interface ApiKeyEntry {
  provider: string;
  has_key: boolean;
}

interface CustomProviderSummary {
  id: number;
  name: string;
  provider_key: string;
  model_id: string;
  has_key: boolean;
}

interface UserInfo {
  id: number;
  username: string;
  first_name?: string | null;
  last_name?: string | null;
  role?: string | null;
  location?: string | null;
}

interface SettingsPanelProps {
  user: UserInfo;
  onClose: () => void;
  onLogout: () => void;
  onCustomProvidersChange?: () => void;
  onUserUpdate?: (user: UserInfo) => void;
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

export function SettingsPanel({ user, onClose, onLogout, onCustomProvidersChange, onUserUpdate }: SettingsPanelProps) {
  const [apiKeys, setApiKeys] = useState<ApiKeyEntry[]>([]);
  const [editingProvider, setEditingProvider] = useState<string | null>(null);
  const [newKeyValue, setNewKeyValue] = useState("");
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [successMessage, setSuccessMessage] = useState<string | null>(null);

  // User profile state
  const [editingProfile, setEditingProfile] = useState(false);
  const [profileForm, setProfileForm] = useState({
    firstName: user.first_name || "",
    lastName: user.last_name || "",
    role: user.role || "",
    location: user.location || "",
  });

  // Custom providers state
  const [customProviders, setCustomProviders] = useState<CustomProviderSummary[]>([]);
  const [showAddCustom, setShowAddCustom] = useState(false);
  const [editingCustomProvider, setEditingCustomProvider] = useState<string | null>(null);
  const [customKeyValue, setCustomKeyValue] = useState("");
  const [newCustomProvider, setNewCustomProvider] = useState({
    name: "",
    endpointUrl: "",
    modelId: "",
    apiKeyHeader: "Authorization",
  });

  useEffect(() => {
    loadApiKeys();
    loadCustomProviders();
  }, []);

  const loadApiKeys = async () => {
    try {
      const keys = await invoke<ApiKeyEntry[]>("list_provider_keys");
      setApiKeys(keys);
    } catch (err) {
      setError(`Failed to load API keys: ${err}`);
    }
  };

  const loadCustomProviders = async () => {
    try {
      const providers = await invoke<CustomProviderSummary[]>("list_custom_providers");
      setCustomProviders(providers);
    } catch (err) {
      console.log("[DEBUG] Could not load custom providers:", err);
    }
  };

  const handleAddCustomProvider = async () => {
    if (!newCustomProvider.name.trim() || !newCustomProvider.endpointUrl.trim() || !newCustomProvider.modelId.trim()) {
      setError("Please fill in all required fields");
      return;
    }

    setIsSaving(true);
    setError(null);

    try {
      await invoke("add_custom_provider", {
        name: newCustomProvider.name.trim(),
        endpointUrl: newCustomProvider.endpointUrl.trim(),
        modelId: newCustomProvider.modelId.trim(),
        apiKeyHeader: newCustomProvider.apiKeyHeader.trim() || "Authorization",
      });
      setSuccessMessage(`Custom provider "${newCustomProvider.name}" added!`);
      setShowAddCustom(false);
      setNewCustomProvider({ name: "", endpointUrl: "", modelId: "", apiKeyHeader: "Authorization" });
      await loadCustomProviders();
      onCustomProvidersChange?.();

      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Failed to add provider: ${err}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleSaveCustomKey = async (providerKey: string) => {
    if (!customKeyValue.trim()) {
      setError("API key cannot be empty");
      return;
    }

    setIsSaving(true);
    setError(null);

    try {
      await invoke("store_custom_provider_key", {
        providerKey,
        apiKey: customKeyValue.trim(),
      });
      const provider = customProviders.find(p => p.provider_key === providerKey);
      setSuccessMessage(`${provider?.name || providerKey} API key saved!`);
      setEditingCustomProvider(null);
      setCustomKeyValue("");
      await loadCustomProviders();

      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Failed to save key: ${err}`);
    } finally {
      setIsSaving(false);
    }
  };

  const handleDeleteCustomProvider = async (providerId: number, providerName: string) => {
    if (!confirm(`Delete custom provider "${providerName}"? This will also delete its API key.`)) {
      return;
    }

    try {
      await invoke("delete_custom_provider", { providerId });
      setSuccessMessage(`${providerName} deleted`);
      await loadCustomProviders();
      onCustomProvidersChange?.();

      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Failed to delete provider: ${err}`);
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

  const handleSaveProfile = async () => {
    setIsSaving(true);
    setError(null);

    try {
      const updatedUser = await invoke<UserInfo>("update_user_profile", {
        firstName: profileForm.firstName.trim() || null,
        lastName: profileForm.lastName.trim() || null,
        role: profileForm.role.trim() || null,
        location: profileForm.location.trim() || null,
      });

      setSuccessMessage("Profile updated successfully!");
      setEditingProfile(false);
      onUserUpdate?.(updatedUser);

      setTimeout(() => setSuccessMessage(null), 3000);
    } catch (err) {
      setError(`Failed to save profile: ${err}`);
    } finally {
      setIsSaving(false);
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

            {/* Profile Information */}
            <div className="profile-section">
              <div className="profile-header">
                <h4>Profile Information</h4>
                {!editingProfile && (
                  <button
                    className="edit-btn"
                    onClick={() => setEditingProfile(true)}
                  >
                    Edit Profile
                  </button>
                )}
              </div>

              {editingProfile ? (
                <div className="profile-edit-form">
                  <div className="profile-field">
                    <label>First Name</label>
                    <input
                      type="text"
                      value={profileForm.firstName}
                      onChange={(e) => setProfileForm({ ...profileForm, firstName: e.target.value })}
                      placeholder="Enter first name"
                      disabled={isSaving}
                    />
                  </div>
                  <div className="profile-field">
                    <label>Last Name</label>
                    <input
                      type="text"
                      value={profileForm.lastName}
                      onChange={(e) => setProfileForm({ ...profileForm, lastName: e.target.value })}
                      placeholder="Enter last name"
                      disabled={isSaving}
                    />
                  </div>
                  <div className="profile-field">
                    <label>Role / Title</label>
                    <input
                      type="text"
                      value={profileForm.role}
                      onChange={(e) => setProfileForm({ ...profileForm, role: e.target.value })}
                      placeholder="e.g., Sales Director, Account Executive"
                      disabled={isSaving}
                    />
                  </div>
                  <div className="profile-field">
                    <label>Location</label>
                    <input
                      type="text"
                      value={profileForm.location}
                      onChange={(e) => setProfileForm({ ...profileForm, location: e.target.value })}
                      placeholder="e.g., New York, NY"
                      disabled={isSaving}
                    />
                  </div>
                  <div className="profile-actions">
                    <button
                      className="save-btn"
                      onClick={handleSaveProfile}
                      disabled={isSaving}
                    >
                      {isSaving ? "Saving..." : "Save Profile"}
                    </button>
                    <button
                      className="cancel-btn"
                      onClick={() => {
                        setEditingProfile(false);
                        setProfileForm({
                          firstName: user.first_name || "",
                          lastName: user.last_name || "",
                          role: user.role || "",
                          location: user.location || "",
                        });
                        setError(null);
                      }}
                      disabled={isSaving}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              ) : (
                <div className="profile-display">
                  <div className="profile-row">
                    <span className="profile-label">Name:</span>
                    <span className="profile-value">
                      {user.first_name || user.last_name
                        ? `${user.first_name || ""} ${user.last_name || ""}`.trim()
                        : "Not set"}
                    </span>
                  </div>
                  <div className="profile-row">
                    <span className="profile-label">Role:</span>
                    <span className="profile-value">{user.role || "Not set"}</span>
                  </div>
                  <div className="profile-row">
                    <span className="profile-label">Location:</span>
                    <span className="profile-value">{user.location || "Not set"}</span>
                  </div>
                </div>
              )}
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

          {/* Custom Providers Section */}
          <div className="settings-section">
            <h3>Custom Providers</h3>
            <p className="section-description">
              Add custom LLM providers with OpenAI-compatible APIs (e.g., Ollama, local models, other providers).
            </p>

            {/* Add Custom Provider Button/Form */}
            {!showAddCustom ? (
              <button
                className="edit-btn"
                onClick={() => setShowAddCustom(true)}
                style={{ marginBottom: "15px" }}
              >
                + Add Custom Provider
              </button>
            ) : (
              <div className="api-key-item" style={{ marginBottom: "15px" }}>
                <div className="api-key-edit" style={{ gap: "10px" }}>
                  <input
                    type="text"
                    value={newCustomProvider.name}
                    onChange={(e) => setNewCustomProvider({ ...newCustomProvider, name: e.target.value })}
                    placeholder="Provider Name (e.g., Ollama, LM Studio)"
                    disabled={isSaving}
                  />
                  <input
                    type="text"
                    value={newCustomProvider.endpointUrl}
                    onChange={(e) => setNewCustomProvider({ ...newCustomProvider, endpointUrl: e.target.value })}
                    placeholder="API Endpoint URL (e.g., http://localhost:11434/v1)"
                    disabled={isSaving}
                  />
                  <input
                    type="text"
                    value={newCustomProvider.modelId}
                    onChange={(e) => setNewCustomProvider({ ...newCustomProvider, modelId: e.target.value })}
                    placeholder="Model ID (e.g., llama3:70b, mistral)"
                    disabled={isSaving}
                  />
                  <input
                    type="text"
                    value={newCustomProvider.apiKeyHeader}
                    onChange={(e) => setNewCustomProvider({ ...newCustomProvider, apiKeyHeader: e.target.value })}
                    placeholder="API Key Header (default: Authorization)"
                    disabled={isSaving}
                  />
                  <div className="api-key-actions">
                    <button
                      className="save-btn"
                      onClick={handleAddCustomProvider}
                      disabled={isSaving}
                    >
                      {isSaving ? "Adding..." : "Add Provider"}
                    </button>
                    <button
                      className="cancel-btn"
                      onClick={() => {
                        setShowAddCustom(false);
                        setNewCustomProvider({ name: "", endpointUrl: "", modelId: "", apiKeyHeader: "Authorization" });
                        setError(null);
                      }}
                      disabled={isSaving}
                    >
                      Cancel
                    </button>
                  </div>
                </div>
              </div>
            )}

            {/* Custom Providers List */}
            {customProviders.length > 0 && (
              <div className="api-keys-list">
                {customProviders.map((provider) => (
                  <div key={provider.id} className="api-key-item">
                    <div className="api-key-header">
                      <div className="api-key-info">
                        <span className="provider-name">{provider.name}</span>
                        <span className="provider-description">Model: {provider.model_id}</span>
                      </div>
                      <span className={`key-status ${provider.has_key ? "configured" : "not-configured"}`}>
                        {provider.has_key ? "Configured" : "Not Set"}
                      </span>
                    </div>

                    {editingCustomProvider === provider.provider_key ? (
                      <div className="api-key-edit">
                        <input
                          type="password"
                          value={customKeyValue}
                          onChange={(e) => setCustomKeyValue(e.target.value)}
                          placeholder="API Key (leave blank if not required)"
                          disabled={isSaving}
                          autoFocus
                        />
                        <div className="api-key-actions">
                          <button
                            className="save-btn"
                            onClick={() => handleSaveCustomKey(provider.provider_key)}
                            disabled={isSaving}
                          >
                            {isSaving ? "Saving..." : "Save"}
                          </button>
                          <button
                            className="cancel-btn"
                            onClick={() => {
                              setEditingCustomProvider(null);
                              setCustomKeyValue("");
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
                            setEditingCustomProvider(provider.provider_key);
                            setCustomKeyValue("");
                            setError(null);
                          }}
                        >
                          {provider.has_key ? "Update Key" : "Add Key"}
                        </button>
                        <button
                          className="delete-btn"
                          onClick={() => handleDeleteCustomProvider(provider.id, provider.name)}
                        >
                          Delete
                        </button>
                      </div>
                    )}
                  </div>
                ))}
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
