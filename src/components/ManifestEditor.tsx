import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { save, open } from "@tauri-apps/plugin-dialog";
import "./ManifestEditor.css";

type PhaseInfo = {
  id: string;
  name: string;
};

type ManifestEditorProps = {
  isOpen: boolean;
  onClose: () => void;
  currentManifestPath: string | null;
  onManifestLoaded: (path: string, name: string, phases: PhaseInfo[]) => void;
  onManifestSaved: (path: string, name: string) => void;
  onRemoveManifest?: (path: string) => void;
};

export function ManifestEditor({
  isOpen,
  onClose,
  currentManifestPath,
  onManifestLoaded,
  onManifestSaved,
  onRemoveManifest,
}: ManifestEditorProps) {
  const [content, setContent] = useState("");
  const [filePath, setFilePath] = useState<string | null>(null);
  const [fileName, setFileName] = useState("Untitled Manifest");
  const [displayName, setDisplayName] = useState(""); // User-editable display name
  const [isModified, setIsModified] = useState(false);
  const [validationStatus, setValidationStatus] = useState<"valid" | "invalid" | "unchecked">("unchecked");
  const [validationMessage, setValidationMessage] = useState("");
  const [validatedPhases, setValidatedPhases] = useState<PhaseInfo[]>([]);
  const [isLoading, setIsLoading] = useState(false);
  const [isSaving, setIsSaving] = useState(false);

  // Load current manifest content when opening editor with existing manifest
  useEffect(() => {
    if (isOpen && currentManifestPath) {
      loadManifestContent(currentManifestPath);
    } else if (isOpen && !currentManifestPath) {
      // New manifest - start with template
      handleNewManifest();
    }
  }, [isOpen, currentManifestPath]);

  // Load manifest content from file
  const loadManifestContent = async (path: string) => {
    setIsLoading(true);
    try {
      const fileContent = await invoke<string>("load_manifest_file", { path });
      setContent(fileContent);
      setFilePath(path);
      const extractedFileName = path.split(/[/\\]/).pop() || "Loaded Manifest";
      setFileName(extractedFileName);
      // Get the actual manifest name from YAML content (manifest.name field)
      try {
        const manifestNameFromYaml = await invoke<string>("get_manifest_name", { path });
        setDisplayName(manifestNameFromYaml);
      } catch {
        // Fallback to filename without extension if YAML name extraction fails
        const nameWithoutExt = extractedFileName.replace(/\.(yaml|yml)$/i, "").replace(/_/g, " ");
        setDisplayName(nameWithoutExt);
      }
      setIsModified(false);
      await validateContent(fileContent);
    } catch (error) {
      console.error("Failed to load manifest:", error);
      setValidationStatus("invalid");
      setValidationMessage(`Failed to load: ${error}`);
    } finally {
      setIsLoading(false);
    }
  };

  // Validate YAML content - returns true if valid, false otherwise
  const validateContent = async (yamlContent: string): Promise<boolean> => {
    if (!yamlContent.trim()) {
      setValidationStatus("unchecked");
      setValidationMessage("Enter manifest content to validate");
      setValidatedPhases([]);
      return false;
    }

    try {
      const phases = await invoke<PhaseInfo[]>("validate_manifest", { content: yamlContent });
      setValidationStatus("valid");
      setValidationMessage(`Valid manifest with ${phases.length} phases`);
      setValidatedPhases(phases);
      return true;
    } catch (error) {
      setValidationStatus("invalid");
      setValidationMessage(`${error}`);
      setValidatedPhases([]);
      return false;
    }
  };

  // Handle content changes
  const handleContentChange = (newContent: string) => {
    setContent(newContent);
    setIsModified(true);
    setValidationStatus("unchecked");
    setValidationMessage("Content modified - click Validate to check");
  };

  // Handle validate button
  const handleValidate = async () => {
    await validateContent(content);
  };

  // Handle new manifest
  const handleNewManifest = async () => {
    try {
      const template = await invoke<string>("get_default_manifest_template");
      setContent(template);
      setFilePath(null);
      setFileName("New Manifest");
      setDisplayName("New Manifest");
      setIsModified(true);
      await validateContent(template);
    } catch (error) {
      console.error("Failed to get template:", error);
    }
  };

  // Handle open file
  const handleOpenFile = async () => {
    try {
      const selected = await open({
        multiple: false,
        filters: [{ name: "YAML Files", extensions: ["yaml", "yml"] }]
      });

      if (selected && typeof selected === "string") {
        await loadManifestContent(selected);
      }
    } catch (error) {
      console.error("Failed to open file:", error);
    }
  };

  // Handle save
  const handleSave = async () => {
    if (validationStatus !== "valid") {
      const isValid = await validateContent(content);
      if (!isValid) {
        alert("Cannot save invalid manifest. Please fix errors first.");
        return;
      }
    }

    setIsSaving(true);
    try {
      let savePath = filePath;

      // If no path, prompt for save location
      if (!savePath) {
        const selectedPath = await save({
          defaultPath: "new_manifest.yaml",
          filters: [{ name: "YAML Files", extensions: ["yaml", "yml"] }]
        });

        if (!selectedPath) {
          setIsSaving(false);
          return; // User cancelled
        }
        savePath = selectedPath;
      }

      // Save the file
      await invoke("save_manifest_file", { path: savePath, content });

      // Update local state
      setFilePath(savePath);
      const newFileName = savePath.split(/[/\\]/).pop() || "Saved Manifest";
      setFileName(newFileName);
      setIsModified(false);

      // Use displayName if set, otherwise derive from filename
      const nameForList = displayName.trim() || newFileName.replace(/\.(yaml|yml)$/i, "").replace(/_/g, " ");
      if (!displayName.trim()) {
        setDisplayName(nameForList);
      }

      // Add to saved manifests list with user's display name
      await invoke("save_manifest_to_list", { name: nameForList, path: savePath });

      // Notify parent
      onManifestSaved(savePath, nameForList);

    } catch (error) {
      console.error("Failed to save manifest:", error);
      alert(`Failed to save: ${error}`);
    } finally {
      setIsSaving(false);
    }
  };

  // Handle save as
  const handleSaveAs = async () => {
    if (validationStatus !== "valid") {
      const isValid = await validateContent(content);
      if (!isValid) {
        alert("Cannot save invalid manifest. Please fix errors first.");
        return;
      }
    }

    setIsSaving(true);
    try {
      const selectedPath = await save({
        defaultPath: fileName.endsWith(".yaml") || fileName.endsWith(".yml")
          ? fileName
          : `${fileName.replace(/\.[^.]+$/, "")}.yaml`,
        filters: [{ name: "YAML Files", extensions: ["yaml", "yml"] }]
      });

      if (!selectedPath) {
        setIsSaving(false);
        return;
      }

      await invoke("save_manifest_file", { path: selectedPath, content });

      setFilePath(selectedPath);
      const newFileName = selectedPath.split(/[/\\]/).pop() || "Saved Manifest";
      setFileName(newFileName);
      setIsModified(false);

      // Use displayName if set, otherwise derive from filename
      const nameForList = displayName.trim() || newFileName.replace(/\.(yaml|yml)$/i, "").replace(/_/g, " ");
      if (!displayName.trim()) {
        setDisplayName(nameForList);
      }

      await invoke("save_manifest_to_list", { name: nameForList, path: selectedPath });
      onManifestSaved(selectedPath, nameForList);

    } catch (error) {
      console.error("Failed to save manifest:", error);
      alert(`Failed to save: ${error}`);
    } finally {
      setIsSaving(false);
    }
  };

  // Handle use manifest (load into app and close editor)
  const handleUseManifest = async () => {
    if (validationStatus !== "valid") {
      const isValid = await validateContent(content);
      if (!isValid) {
        alert("Cannot use invalid manifest. Please fix errors first.");
        return;
      }
    }

    // If modified, prompt to save first
    if (isModified) {
      const shouldSave = confirm("You have unsaved changes. Save before using this manifest?");
      if (shouldSave) {
        await handleSave();
      }
    }

    // If we have a valid path and valid content, load it
    if (filePath && validatedPhases.length > 0) {
      const nameToUse = displayName.trim() || fileName.replace(/\.(yaml|yml)$/i, "").replace(/_/g, " ");
      onManifestLoaded(filePath, nameToUse, validatedPhases);
      onClose();
    } else if (!filePath && validatedPhases.length > 0) {
      alert("Please save the manifest first before using it.");
    }
  };

  // Handle close with unsaved changes check
  const handleClose = () => {
    if (isModified) {
      const shouldClose = confirm("You have unsaved changes. Discard them?");
      if (!shouldClose) return;
    }
    onClose();
  };

  if (!isOpen) return null;

  return (
    <div className="manifest-editor-overlay">
      <div className="manifest-editor-panel">
        {/* Header */}
        <div className="manifest-editor-header">
          <h2>
            Manifest Editor
            {isModified && <span className="modified-indicator">*</span>}
          </h2>
          <button className="close-btn" onClick={handleClose}>×</button>
        </div>

        {/* Toolbar */}
        <div className="manifest-editor-toolbar">
          <div className="toolbar-group">
            <button className="toolbar-btn" onClick={handleNewManifest} title="Create new manifest">
              <span className="btn-icon">+</span> New
            </button>
            <button className="toolbar-btn" onClick={handleOpenFile} title="Open existing file">
              Open
            </button>
          </div>
          <div className="toolbar-group">
            <button
              className="toolbar-btn"
              onClick={handleSave}
              disabled={isSaving || validationStatus === "invalid"}
              title="Save manifest"
            >
              Save
            </button>
            <button
              className="toolbar-btn"
              onClick={handleSaveAs}
              disabled={isSaving || validationStatus === "invalid"}
              title="Save to new location"
            >
              Save As…
            </button>
          </div>
          <div className="toolbar-separator"></div>
          <button
            className="toolbar-btn validate-btn"
            onClick={handleValidate}
            title="Validate YAML syntax"
          >
            Validate
          </button>
          <button
            className="toolbar-btn use-btn"
            onClick={handleUseManifest}
            disabled={validationStatus !== "valid"}
            title="Load manifest and start research"
          >
            Use Manifest
          </button>
          {filePath && onRemoveManifest && (
            <>
              <div className="toolbar-separator"></div>
              <button
                className="toolbar-btn remove-btn"
                onClick={() => {
                  if (confirm(`Remove "${displayName || fileName}" from your saved manifests?`)) {
                    onRemoveManifest(filePath);
                    onClose();
                  }
                }}
                title="Remove from saved manifests"
              >
                Remove
              </button>
            </>
          )}
        </div>

        {/* File info bar */}
        <div className="manifest-editor-info">
          <div className="display-name-row">
            <label className="display-name-label">Name:</label>
            <input
              type="text"
              className="display-name-input"
              value={displayName}
              onChange={(e) => {
                setDisplayName(e.target.value);
              }}
              placeholder="Enter manifest name..."
            />
            {filePath && (
              <button
                className="rename-btn"
                onClick={async () => {
                  if (!displayName.trim()) {
                    alert("Please enter a name");
                    return;
                  }
                  try {
                    await invoke("save_manifest_to_list", { name: displayName.trim(), path: filePath });
                    onManifestSaved(filePath, displayName.trim());
                  } catch (error) {
                    console.error("Failed to rename:", error);
                    alert(`Failed to rename: ${error}`);
                  }
                }}
                title="Apply name change"
              >
                Rename
              </button>
            )}
          </div>
          {filePath && <span className="file-path" title={filePath}>{filePath}</span>}
        </div>

        {/* Main content area */}
        <div className="manifest-editor-content">
          {/* Editor */}
          <div className="editor-pane">
            {isLoading ? (
              <div className="loading-overlay">Loading manifest...</div>
            ) : (
              <textarea
                className="yaml-editor"
                value={content}
                onChange={(e) => handleContentChange(e.target.value)}
                placeholder="Enter your manifest YAML here..."
                spellCheck={false}
              />
            )}
          </div>

          {/* Validation sidebar */}
          <div className="validation-pane">
            <div className={`validation-status status-${validationStatus}`}>
              {validationStatus === "valid" && "✓ Valid"}
              {validationStatus === "invalid" && "✗ Invalid"}
              {validationStatus === "unchecked" && "○ Unchecked"}
            </div>
            <div className="validation-message">{validationMessage}</div>

            {validatedPhases.length > 0 && (
              <div className="phases-preview">
                <h4>Phases ({validatedPhases.length})</h4>
                <ul>
                  {validatedPhases.map((phase, idx) => (
                    <li key={phase.id}>
                      <span className="phase-number">{idx + 1}.</span>
                      <span className="phase-name">{phase.name}</span>
                    </li>
                  ))}
                </ul>
              </div>
            )}
          </div>
        </div>
      </div>
    </div>
  );
}
