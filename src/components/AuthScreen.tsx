import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./AuthScreen.css";
import fullintelLogoWide from "../assets/fullintel_logo_wide.jpg";

interface UserInfo {
  id: number;
  username: string;
}

interface AuthScreenProps {
  onLoginSuccess: (user: UserInfo) => void;
}

export function AuthScreen({ onLoginSuccess }: AuthScreenProps) {
  const [mode, setMode] = useState<"login" | "register">("login");
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [confirmPassword, setConfirmPassword] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isLoading, setIsLoading] = useState(false);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setError(null);

    if (!username.trim() || !password.trim()) {
      setError("Username and password are required");
      return;
    }

    if (mode === "register") {
      if (password !== confirmPassword) {
        setError("Passwords do not match");
        return;
      }
      if (password.length < 8) {
        setError("Password must be at least 8 characters");
        return;
      }
    }

    setIsLoading(true);

    try {
      if (mode === "register") {
        // Register new user
        await invoke<UserInfo>("auth_register", {
          username: username.trim(),
          password,
        });
        // Auto-login after registration
      }

      // Login
      const user = await invoke<UserInfo>("auth_login", {
        username: username.trim(),
        password,
      });

      onLoginSuccess(user);
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      if (message.includes("UserExists")) {
        setError("Username already taken. Please choose another.");
      } else if (message.includes("InvalidCredentials")) {
        setError("Invalid username or password.");
      } else {
        setError(message);
      }
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div className="auth-container">
      <div className="auth-card">
        <div className="auth-header">
          <img src={fullintelLogoWide} alt="FullIntel" className="auth-logo" />
          <p>AI-Powered Sales Intelligence</p>
        </div>

        <div className="auth-tabs">
          <button
            className={mode === "login" ? "active" : ""}
            onClick={() => {
              setMode("login");
              setError(null);
            }}
          >
            Sign In
          </button>
          <button
            className={mode === "register" ? "active" : ""}
            onClick={() => {
              setMode("register");
              setError(null);
            }}
          >
            Create Account
          </button>
        </div>

        <form onSubmit={handleSubmit} className="auth-form">
          {error && <div className="auth-error">{error}</div>}

          <div className="auth-field">
            <label htmlFor="username">Username</label>
            <input
              id="username"
              type="text"
              value={username}
              onChange={(e) => setUsername(e.target.value)}
              placeholder="Enter username"
              disabled={isLoading}
              autoComplete="username"
            />
          </div>

          <div className="auth-field">
            <label htmlFor="password">Password</label>
            <input
              id="password"
              type="password"
              value={password}
              onChange={(e) => setPassword(e.target.value)}
              placeholder="Enter password"
              disabled={isLoading}
              autoComplete={mode === "login" ? "current-password" : "new-password"}
            />
          </div>

          {mode === "register" && (
            <div className="auth-field">
              <label htmlFor="confirmPassword">Confirm Password</label>
              <input
                id="confirmPassword"
                type="password"
                value={confirmPassword}
                onChange={(e) => setConfirmPassword(e.target.value)}
                placeholder="Confirm password"
                disabled={isLoading}
                autoComplete="new-password"
              />
            </div>
          )}

          <button type="submit" className="auth-submit" disabled={isLoading}>
            {isLoading
              ? "Please wait..."
              : mode === "login"
              ? "Sign In"
              : "Create Account"}
          </button>

          {/* Mini color palette reference - matches main app */}
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
        </form>

        <div className="auth-footer">
          <p>
            {mode === "login" ? (
              <>
                Don't have an account?{" "}
                <button onClick={() => setMode("register")}>Create one</button>
              </>
            ) : (
              <>
                Already have an account?{" "}
                <button onClick={() => setMode("login")}>Sign in</button>
              </>
            )}
          </p>
        </div>
      </div>
    </div>
  );
}
