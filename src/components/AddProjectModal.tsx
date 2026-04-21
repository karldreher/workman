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

  const displayBranch = branch || "…";
  const displayName = name.trim() || "…";

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">New project</div>
        <div className="modal-hint">Give the project a short name — the branch is derived automatically.</div>
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

        <div className="modal-info-box">
          <div className="modal-info-row">
            <span className="modal-info-icon">⑂</span>
            <span>
              When you add a repo, workman creates the{" "}
              <code className="modal-info-code">{displayBranch}</code> branch (if it doesn't
              exist yet) and checks it out as a git worktree.
            </span>
          </div>
          <div className="modal-info-row">
            <span className="modal-info-icon">⛓</span>
            <span>
              All worktrees live directly at{" "}
              <code className="modal-info-code">~/.workman/projects/{displayName}/&lt;repo&gt;/</code>
              — one central place, no scattered directories inside your repos. Open the
              whole project as a VS Code workspace in one click.
            </span>
          </div>
        </div>

        <div className="modal-hint" style={{ marginTop: 4 }}>
          Enter to confirm · Esc to cancel
        </div>
      </div>
    </div>
  );
}
