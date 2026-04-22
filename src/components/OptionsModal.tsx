import { useEffect, useState } from "react";
import { Settings } from "../api/types";

interface Props {
  /** Current settings shown as initial state. */
  settings: Settings;
  /** Called with the new settings when the user confirms (Enter). */
  onSave: (settings: Settings) => void;
  onClose: () => void;
}

/** Settings panel. Enter saves; Escape cancels. */
export default function OptionsModal({ settings, onSave, onClose }: Props) {
  const [useExternalTerminal, setUseExternalTerminal] = useState(settings.use_external_terminal);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") { e.preventDefault(); onClose(); }
      if (e.key === "Enter") { e.preventDefault(); onSave({ use_external_terminal: useExternalTerminal }); }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onSave, onClose, useExternalTerminal]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Options</div>
        <div className="option-row">
          <div>
            <div className="option-label">Use external terminal</div>
            <div className="option-desc">
              Open the system terminal app instead of the built-in pane.
              When off, sessions are persisted via tmux if available.
            </div>
          </div>
          <button
            className={`toggle${useExternalTerminal ? " on" : ""}`}
            onClick={() => setUseExternalTerminal((v) => !v)}
          >
            {useExternalTerminal ? "on" : "off"}
          </button>
        </div>
        <div className="modal-hint" style={{ marginTop: 12 }}>
          Enter to save · Esc to cancel
        </div>
      </div>
    </div>
  );
}
