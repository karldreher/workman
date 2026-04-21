import { useRef } from "react";
import { usePtySession } from "../hooks/usePtySession";

interface Props {
  sessionId: string;
  workingDir: string;
  onClose: () => void;
}

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
