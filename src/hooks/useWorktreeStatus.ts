import { useCallback, useEffect, useState } from "react";
import { getAllStatuses } from "../api/tauri";

/**
 * Fetches and caches worktree status strings for all known worktrees.
 *
 * @returns `statuses` — map of `"<project>/<repo>"` → status string, and
 *   `refresh` — callback to re-fetch on demand (e.g. after a push).
 */
export function useWorktreeStatus() {
  const [statuses, setStatuses] = useState<Record<string, string>>({});

  const refresh = useCallback(() => {
    getAllStatuses()
      .then(setStatuses)
      .catch(console.error);
  }, []);

  useEffect(() => {
    refresh();
  }, [refresh]);

  return { statuses, refresh };
}
