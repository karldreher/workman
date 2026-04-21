import { useEffect, useRef, useState } from "react";
import { getRepoSuggestions } from "../api/tauri";
import { RepoSuggestion } from "../api/types";

interface Props {
  projectName: string;
  knownRepos: string[];
  onAdd: (repoPath: string) => Promise<void>;
  onClose: () => void;
}

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

  function handleKeyDown(e: React.KeyboardEvent<HTMLInputElement>) {
    if (e.key === "Tab") {
      e.preventDefault();
      const target = cursor !== null ? suggestions[cursor] : suggestions[0];
      if (target) {
        let path = target.path;
        if (!path.endsWith("/")) path += "/";
        setInput(path);
      }
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setCursor((c) => (c === null ? 0 : Math.min(c + 1, suggestions.length - 1)));
      return;
    }
    if (e.key === "ArrowUp") {
      e.preventDefault();
      setCursor((c) => (c === null ? suggestions.length - 1 : Math.max(c - 1, 0)));
      return;
    }
    if (e.key === "Enter") {
      e.preventDefault();
      const path = cursor !== null ? suggestions[cursor]?.path : input;
      handleConfirm(path ?? input);
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
          placeholder="/path/to/repo"
          spellCheck={false}
        />
        {errorMessage && <div className="modal-error">{errorMessage}</div>}
        {suggestions.length > 0 && (
          <div className="suggestion-list">
            {suggestions.map((s, i) => (
              <div
                key={s.path}
                className={`suggestion-item${cursor === i ? " highlighted" : ""}`}
                onMouseEnter={() => setCursor(i)}
                onClick={() => handleConfirm(s.path)}
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
