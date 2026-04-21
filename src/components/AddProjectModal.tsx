import { useEffect, useRef, useState } from "react";
import { branchFromName } from "../api/tauri";

interface Props {
  onConfirm: (name: string) => void;
  onCancel: () => void;
}

export default function AddProjectModal({ onConfirm, onCancel }: Props) {
  const [name, setName] = useState("");
  const [branch, setBranch] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  useEffect(() => {
    if (!name.trim()) { setBranch(""); return; }
    branchFromName(name).then(setBranch).catch(() => setBranch(""));
  }, [name]);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); if (name.trim()) onConfirm(name.trim()); }
    if (e.key === "Escape") { e.preventDefault(); onCancel(); }
  }

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">New project</div>
        <div className="modal-hint">Project name (branch is derived automatically)</div>
        <input
          ref={inputRef}
          className="modal-input"
          value={name}
          onChange={(e) => setName(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="e.g. my feature"
          spellCheck={false}
        />
        {branch && (
          <div className="modal-branch-preview">
            branch: <span>{branch}</span>
          </div>
        )}
        <div className="modal-hint" style={{ marginTop: 12 }}>
          Enter to confirm · Esc to cancel
        </div>
      </div>
    </div>
  );
}
