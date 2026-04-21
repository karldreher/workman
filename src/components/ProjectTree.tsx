import { Config } from "../api/types";

interface Props {
  config: Config;
  statuses: Record<string, string>;
  selectedProject: string | null;
  selectedRepo: string | null;
  expandedProjects: Set<string>;
  onSelectProject: (name: string) => void;
  onSelectRepo: (project: string, repo: string) => void;
  onToggleExpand: (name: string) => void;
}

function statusClass(status: string | undefined): string {
  if (!status || status === "...") return "loading";
  if (status === "clean") return "clean";
  if (status === "N/A") return "missing";
  return "dirty";
}

export default function ProjectTree({
  config,
  statuses,
  selectedProject,
  selectedRepo,
  expandedProjects,
  onSelectProject,
  onSelectRepo,
  onToggleExpand,
}: Props) {
  return (
    <div className="tree-list">
      {config.projects.length === 0 && (
        <div className="tree-item" style={{ color: "var(--text-dim)", fontStyle: "italic" }}>
          No projects — press n
        </div>
      )}
      {config.projects.map((project) => {
        const expanded = expandedProjects.has(project.name);
        const isProjectSelected = selectedProject === project.name && selectedRepo === null;
        return (
          <div key={project.name}>
            <div
              className={`tree-item tree-item-project${isProjectSelected ? " selected" : ""}`}
              onClick={() => {
                onSelectProject(project.name);
                onToggleExpand(project.name);
              }}
            >
              <span className="tree-prefix">{expanded ? "▼" : "▶"}</span>
              <span>{project.name}</span>
              <span className="tree-branch">{project.branch}</span>
            </div>
            {expanded &&
              project.worktrees.map((wt, idx) => {
                const isLast = idx === project.worktrees.length - 1;
                const isSelected = selectedProject === project.name && selectedRepo === wt.repo_name;
                const statusKey = `${project.name}/${wt.repo_name}`;
                const status = statuses[statusKey];
                return (
                  <div
                    key={wt.repo_name}
                    className={`tree-item tree-item-worktree${isSelected ? " selected" : ""}`}
                    onClick={() => onSelectRepo(project.name, wt.repo_name)}
                  >
                    <span className="tree-prefix">{isLast ? "└─" : "├─"}</span>
                    <span className="tree-repo-name">{wt.repo_name}</span>
                    <span className={`tree-status ${statusClass(status)}`}>
                      {status ?? "..."}
                    </span>
                  </div>
                );
              })}
          </div>
        );
      })}
    </div>
  );
}
