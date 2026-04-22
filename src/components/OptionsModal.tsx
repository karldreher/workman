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
  const [useTmux, setUseTmux] = useState(settings.use_tmux);

  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "Escape") { e.preventDefault(); onClose(); }
      if (e.key === "Enter") { e.preventDefault(); onSave({ use_tmux: useTmux }); }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onSave, onClose, useTmux]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Options</div>
        <div className="option-row">
          <div>
            <div className="option-label">use_tmux</div>
            <div className="option-desc">Open external terminal instead of in-app xterm</div>
          </div>
          <button
            className={`toggle${useTmux ? " on" : ""}`}
            onClick={() => setUseTmux((v) => !v)}
          >
            {useTmux ? "on" : "off"}
          </button>
        </div>
        <div className="modal-hint" style={{ marginTop: 12 }}>
          Enter to save · Esc to cancel
        </div>
      </div>
    </div>
  );
}
