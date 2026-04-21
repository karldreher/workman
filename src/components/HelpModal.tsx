import { useEffect } from "react";

interface Props {
  onClose: () => void;
}

const shortcuts = [
  ["n", "new project"],
  ["a", "add repo to project"],
  ["t", "open terminal (or external if use_tmux)"],
  ["p", "push (worktree or project)"],
  ["d", "show diff (worktree)"],
  ["x", "delete project or worktree"],
  ["o", "options"],
  ["↑ / ↓", "navigate"],
  ["Enter", "expand / collapse project"],
  ["Esc", "close panel / dismiss error"],
  ["q", "quit"],
];

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
