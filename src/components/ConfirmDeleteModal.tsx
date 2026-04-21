import { useEffect } from "react";

interface Props {
  message: string;
  onConfirm: () => void;
  onCancel: () => void;
}

export default function ConfirmDeleteModal({ message, onConfirm, onCancel }: Props) {
  useEffect(() => {
    function handleKeyDown(e: KeyboardEvent) {
      if (e.key === "y" || e.key === "Enter") { e.preventDefault(); onConfirm(); }
      if (e.key === "n" || e.key === "Escape") { e.preventDefault(); onCancel(); }
    }
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [onConfirm, onCancel]);

  return (
    <div className="modal-overlay" onClick={onCancel}>
      <div className="modal" onClick={(e) => e.stopPropagation()}>
        <div className="modal-title">Confirm delete</div>
        <div className="modal-confirm-message">{message}</div>
        <div className="modal-actions">
          <button className="btn" onClick={onCancel}>
            [n] Cancel
          </button>
          <button className="btn btn-danger" onClick={onConfirm}>
            [y] Delete
          </button>
        </div>
      </div>
    </div>
  );
}
