import { useCallback, useEffect, useState } from "react";
import { getAllStatuses } from "../api/tauri";

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
