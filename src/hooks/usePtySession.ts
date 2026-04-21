import { useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { WebLinksAddon } from "@xterm/addon-web-links";
import { openPtySession, closePtySession, writeToPty, resizePty } from "../api/tauri";
import "@xterm/xterm/css/xterm.css";

export function usePtySession(
  containerRef: React.RefObject<HTMLDivElement | null>,
  sessionId: string,
  workingDir: string,
) {
  const terminalRef = useRef<Terminal | null>(null);

  useEffect(() => {
    if (!containerRef.current) return;

    const terminal = new Terminal({
      theme: {
        background: "#1c1c1c",
        foreground: "#d4d4d4",
        cursor: "#00d7d7",
        selectionBackground: "#3a3a3a",
        black: "#1c1c1c",
        brightBlack: "#4e4e4e",
        cyan: "#00d7d7",
        brightCyan: "#5fd7ff",
        green: "#5faf5f",
        brightGreen: "#87d75f",
        yellow: "#d7af00",
        brightYellow: "#ffd700",
        red: "#d75f5f",
        brightRed: "#ff5f5f",
        blue: "#5f87d7",
        brightBlue: "#5fafd7",
        magenta: "#af5fd7",
        brightMagenta: "#d75faf",
        white: "#d4d4d4",
        brightWhite: "#ffffff",
      },
      fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace",
      fontSize: 13,
      lineHeight: 1.2,
      cursorBlink: true,
      scrollback: 5000,
    });

    const fitAddon = new FitAddon();
    const webLinksAddon = new WebLinksAddon();
    terminal.loadAddon(fitAddon);
    terminal.loadAddon(webLinksAddon);
    terminal.open(containerRef.current);
    fitAddon.fit();
    terminalRef.current = terminal;

    const { cols, rows } = terminal;
    openPtySession(sessionId, workingDir, cols, rows).catch(console.error);

    const unlistenOutput = listen<{ session_id: string; data: number[] }>(
      "pty-output",
      (event) => {
        if (event.payload.session_id === sessionId) {
          terminal.write(new Uint8Array(event.payload.data));
        }
      },
    );

    const unlistenExit = listen<{ session_id: string }>("pty-exit", (event) => {
      if (event.payload.session_id === sessionId) {
        terminal.write("\r\n\x1b[33m[session ended]\x1b[0m\r\n");
      }
    });

    terminal.onData(async (data) => {
      const bytes = Array.from(new TextEncoder().encode(data));
      await writeToPty(sessionId, bytes).catch(console.error);
    });

    terminal.onResize(({ cols, rows }) => {
      resizePty(sessionId, cols, rows).catch(console.error);
    });

    const resizeObserver = new ResizeObserver(() => {
      fitAddon.fit();
    });
    resizeObserver.observe(containerRef.current);

    // Focus terminal so it receives keyboard input immediately
    terminal.focus();

    return () => {
      unlistenOutput.then((fn) => fn());
      unlistenExit.then((fn) => fn());
      resizeObserver.disconnect();
      terminal.dispose();
      closePtySession(sessionId).catch(console.error);
    };
  }, [sessionId, workingDir]);

  return terminalRef;
}
