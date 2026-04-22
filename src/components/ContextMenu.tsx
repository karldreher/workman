import { useEffect, useRef } from "react";

/** A single entry in a {@link ContextMenu}. Use `type: "separator"` for a visual divider. */
export type MenuItem =
  | { type: "item"; label: string; icon?: string; onClick: () => void; danger?: boolean; disabled?: boolean }
  | { type: "separator" };

interface Props {
  /** Viewport X coordinate for the menu's top-left corner (clamped to stay on screen). */
  x: number;
  /** Viewport Y coordinate for the menu's top-left corner (clamped to stay on screen). */
  y: number;
  items: MenuItem[];
  onClose: () => void;
}

/** Fixed-position context menu. Clamps itself to the viewport after mount and closes on outside click or Escape. */
export default function ContextMenu({ x, y, items, onClose }: Props) {
  const menuRef = useRef<HTMLDivElement>(null);

  // Clamp position so menu stays in viewport
  const style: React.CSSProperties = {
    position: "fixed",
    left: x,
    top: y,
    zIndex: 200,
  };

  useEffect(() => {
    if (!menuRef.current) return;
    const rect = menuRef.current.getBoundingClientRect();
    if (rect.right > window.innerWidth)
      menuRef.current.style.left = `${x - rect.width}px`;
    if (rect.bottom > window.innerHeight)
      menuRef.current.style.top = `${y - rect.height}px`;
  }, [x, y]);

  useEffect(() => {
    const handler = (e: MouseEvent) => {
      if (menuRef.current && !menuRef.current.contains(e.target as Node)) onClose();
    };
    const escHandler = (e: KeyboardEvent) => { if (e.key === "Escape") onClose(); };
    document.addEventListener("mousedown", handler);
    document.addEventListener("keydown", escHandler);
    return () => {
      document.removeEventListener("mousedown", handler);
      document.removeEventListener("keydown", escHandler);
    };
  }, [onClose]);

  return (
    <div ref={menuRef} className="context-menu" style={style}>
      {items.map((item, i) =>
        item.type === "separator" ? (
          <div key={i} className="context-menu-separator" />
        ) : (
          <button
            key={i}
            className={`context-menu-item${item.danger ? " danger" : ""}${item.disabled ? " disabled" : ""}`}
            onClick={() => { if (!item.disabled) { item.onClick(); onClose(); } }}
            disabled={item.disabled}
          >
            {item.icon && <span className="context-menu-icon">{item.icon}</span>}
            {item.label}
          </button>
        )
      )}
    </div>
  );
}
