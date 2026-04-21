import { useEffect } from "react";

const isMac = /Mac|iPhone|iPad/.test(navigator.platform);
const mod = isMac ? "⌥" : "Alt+";

const shortcuts = [
  [`${mod}N`, "new project"],
  [`${mod}A`, "add repo to project"],
  [`${mod}T`, "open terminal"],
  [`${mod}P`, "push (worktree or project)"],
  [`${mod}D`, "show diff"],
  [`${mod}X`, "delete project or worktree"],
  [`${mod}O`, "options"],
  [`${mod}H`, "help"],
  [`${mod}Q`, "quit"],
  ["↑ / ↓", "navigate tree"],
  ["Enter", "expand / collapse project"],
  ["Esc", "close panel / dismiss error"],
];

interface Props {
  onClose: () => void;
}

export default function HelpModal({ onClose }: Props) {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      e.preventDefault();
      onClose();
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onClose]);

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Keyboard shortcuts</div>
        <table className="help-table">
          <tbody>
            {shortcuts.map(([key, desc]) => (
              <tr key={key}>
                <td>{key}</td>
                <td>{desc}</td>
              </tr>
            ))}
          </tbody>
        </table>
        <div className="modal-hint" style={{ marginTop: 12 }}>
          Any key to close
        </div>
      </div>
    </div>
  );
}
