import { useRef } from "react";
import { usePtySession } from "../hooks/usePtySession";

interface Props {
  /** Unique PTY session identifier. Passed through to {@link usePtySession}. */
  sessionId: string;
  /** Absolute path shown in the header and used as the shell's working directory. */
  workingDir: string;
  /** Called when the user clicks "✕ detach" — does NOT kill the shell session. */
  onClose: () => void;
}

/** Renders an xterm.js terminal connected to a backend PTY session. Detach keeps the shell alive. */
export default function TerminalPane({ sessionId, workingDir, onClose }: Props) {
  const containerRef = useRef<HTMLDivElement>(null);
  usePtySession(containerRef, sessionId, workingDir);

  return (
    <div style={{ display: "flex", flexDirection: "column", height: "100%" }}>
      <div className="terminal-header">
        <span className="terminal-header-path">{workingDir}</span>
        <button className="terminal-close" onClick={onClose}>
          ✕ detach
        </button>
      </div>
      <div ref={containerRef} className="terminal-container" />
    </div>
  );
}
