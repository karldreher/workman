import { useEffect, useRef, useState } from "react";

interface Props {
  /** Display label for what's being pushed (e.g. `"frontend"` or `"my-feature"`). */
  scope: string;
  /** Called with the trimmed message, or `undefined` to use the default auto-commit message. */
  onConfirm: (message?: string) => void;
  onCancel: () => void;
}

/** Modal that prompts for an optional commit message before a push. */
export default function CommitMessageModal({ scope, onConfirm, onCancel }: Props) {
  const [message, setMessage] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  function handleKeyDown(e: React.KeyboardEvent) {
    if (e.key === "Enter") { e.preventDefault(); onConfirm(message.trim() || undefined); }
    if (e.key === "Escape") { e.preventDefault(); onCancel(); }
  }

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Push {scope}</div>
        <div className="modal-hint">Commit message (leave empty for default)</div>
        <input
          ref={inputRef}
          className="modal-input"
          value={message}
          onChange={(e) => setMessage(e.target.value)}
          onKeyDown={handleKeyDown}
          placeholder="workman: auto-commit"
          spellCheck={false}
        />
        <div className="modal-hint" style={{ marginTop: 12 }}>
          Enter to push · Esc to cancel
        </div>
      </div>
    </div>
  );
}
