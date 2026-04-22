import { useEffect, useRef, useState } from "react";
import { getRepoSuggestions } from "../api/tauri";
import { RepoSuggestion } from "../api/types";

interface Props {
  /** Name of the project repos are being added to (shown in the modal title). */
  projectName: string;
  /** Paths of repos already added to this project (unused currently; reserved for future deduplication). */
  knownRepos: string[];
  /**
   * Called with the absolute path of a validated git repo.
   * Should throw with a user-facing error string if the add fails.
   */
  onAdd: (repoPath: string) => Promise<void>;
  onClose: () => void;
}

/**
 * Repo picker modal with filesystem autocomplete.
 * Starts at the user's home directory. Known repos are highlighted and shown only when the input is empty.
 * Non-git directories navigate (append `/`) instead of adding.
 */
export default function AddRepoModal({ projectName, onAdd, onClose }: Props) {
  const [input, setInput] = useState("");
  const [suggestions, setSuggestions] = useState<RepoSuggestion[]>([]);
  const [cursor, setCursor] = useState<number | null>(null);
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    getRepoSuggestions(input).then(setSuggestions).catch(() => setSuggestions([]));
    setCursor(null);
  }, [input]);

  const visibleSuggestions = input === ""
    ? suggestions
    : suggestions.filter((s) => !s.known);

  async function handleConfirm(path: string) {
    if (!path.trim()) { onClose(); return; }
    try {
      setErrorMessage(null);
      await onAdd(path.trim());
      setInput("");
    } catch (e) {
      setErrorMessage(String(e));
    }
  }

  function navigateTo(path: string) {
    if (!path.endsWith("/")) path += "/";
    setInput(path);
  }

  function handleSuggestionClick(s: RepoSuggestion) {
    if (!s.is_git_repo) {
      navigateTo(s.path);
    } else {
      handleConfirm(s.path);
    }
  }

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Tab") {
      e.preventDefault();
      const target = cursor !== null ? visibleSuggestions[cursor] : visibleSuggestions[0];
      if (target) navigateTo(target.path);
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setCursor((c) => (c === null ? 0 : Math.min(c + 1, visibleSuggestions.length - 1)));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setCursor((c) => (c === null ? visibleSuggestions.length - 1 : Math.max(c - 1, 0)));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      if (cursor !== null && visibleSuggestions[cursor]) {
        const s = visibleSuggestions[cursor];
        if (!s.is_git_repo) { navigateTo(s.path); return; }
        handleConfirm(s.path);
      } else {
        handleConfirm(input);
      }
      return;
    }
    if (e.key === "Escape") {
      e.preventDefault();
      onClose();
    }
  }

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Add repo to {projectName}</div>
        <div className="modal-hint">
          Tab to complete · ↑↓ to navigate · Enter to add · Enter empty to finish
        </div>
        <input
          ref={inputRef}
          className="modal-input"
          value={input}
          onChange={(e) => setInput(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="Type to filter…"
          spellCheck={false}
        />
        {errorMessage && <div className="modal-error">{errorMessage}</div>}
        {visibleSuggestions.length > 0 && (
          <div className="suggestion-list">
            {visibleSuggestions.map((s, i) => (
              <div
                key={s.path}
                className={[
                  "suggestion-item",
                  cursor === i ? "highlighted" : "",
                  s.known ? "suggestion-known-repo" : "",
                  !s.is_git_repo ? "suggestion-non-git" : "",
                ].filter(Boolean).join(" ")}
                onMouseEnter={() => setCursor(i)}
                onClick={() => handleSuggestionClick(s)}
              >
                {s.known && <span className="suggestion-known">★</span>}
                <span className="suggestion-path">{s.path}</span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
