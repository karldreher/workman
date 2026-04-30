import { useEffect, useState } from "react";

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
  const [tab, setTab] = useState<"about" | "shortcuts">("about");

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
        <div className="modal-tabs">
          <button
            className={`modal-tab${tab === "about" ? " active" : ""}`}
            onClick={() => setTab("about")}
          >
            about
          </button>
          <button
            className={`modal-tab${tab === "shortcuts" ? " active" : ""}`}
            onClick={() => setTab("shortcuts")}
          >
            shortcuts
          </button>
        </div>

        {tab === "about" && (
          <div className="help-about">
            <p>
              workman manages git worktrees across multiple repositories.
              Create a <strong>project</strong> once and workman provisions
              matching worktrees in every registered repo — all on the same
              branch, ready to work in parallel.
            </p>
            <p>
              From any project you can open a terminal, inspect a diff,
              and commit and push every repo in one action — no branch
              switching, no stashing.
            </p>
            <div className="help-about-detail">
              <div><span>projects</span> named groupings of worktrees sharing a branch</div>
              <div><span>repos</span> git repositories registered once, reused across projects</div>
              <div><span>worktrees</span> checkouts at <code>~/.workman/projects/&lt;project&gt;/&lt;repo&gt;/</code></div>
            </div>
          </div>
        )}

        {tab === "shortcuts" && (
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
        )}

        <div className="modal-hint" style={{ marginTop: 12 }}>
          Any key to close
        </div>
      </div>
    </div>
  );
}
