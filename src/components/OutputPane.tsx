interface Props {
  /** Lines to display. Empty string entries render as a non-breaking space to preserve line height. */
  lines: string[];
  /** When `true`, lines are colorized as a unified diff (added/removed/hunk/header). */
  isDiff: boolean;
  onClose: () => void;
}

/** Maps a diff line to its CSS class for syntax coloring. */
function lineClass(line: string, isDiff: boolean): string {
  if (!isDiff) return "output-line";
  if (line.startsWith("+") && !line.startsWith("+++")) return "output-line diff-add";
  if (line.startsWith("-") && !line.startsWith("---")) return "output-line diff-del";
  if (line.startsWith("@@")) return "output-line diff-hunk";
  if (line.startsWith("diff ") || line.startsWith("index ") || line.startsWith("---") || line.startsWith("+++"))
    return "output-line diff-header";
  return "output-line";
}

export default function OutputPane({ lines, isDiff, onClose }: Props) {
  return (
    <div className="output-pane">
      <div className="output-header">
        <span className="output-header-title">{isDiff ? "diff" : "output"}</span>
        <button className="output-close" onClick={onClose}>✕</button>
      </div>
      <div className="output-scroll">
        {lines.map((line, i) => (
          <div key={i} className={lineClass(line, isDiff)}>
            {line || "\u00a0"}
          </div>
        ))}
        {lines.length === 0 && (
          <div className="output-line" style={{ color: "var(--text-dim)" }}>(empty)</div>
        )}
      </div>
    </div>
  );
}
