import { Config } from "../api/types";

interface Props {
  config: Config;
  /** Map of `"<project>/<repo>"` → status string (e.g. `"clean"`, `"5/-3"`, `"N/A"`). */
  statuses: Record<string, string>;
  selectedProject: string | null;
  selectedRepo: string | null;
  /** Set of project names whose worktrees are currently expanded in the tree. */
  expandedProjects: Set<string>;
  /** Whether the Alt/Option key is currently held — reveals keyboard shortcut hints. */
  altHeld: boolean;
  /** `true` on macOS — affects how shortcut hints are rendered (⌥ vs `Alt+`). */
  isMac: boolean;
  onSelectProject: (name: string) => void;
  onSelectRepo: (project: string, repo: string) => void;
  onToggleExpand: (name: string) => void;
  onNewProject: () => void;
  onAddRepo: (projectName: string) => void;
  onTerminal: (projectName: string, repoName?: string) => void;
  onPush: (projectName: string, repoName?: string) => void;
  onDiff: (projectName: string, repoName: string) => void;
  onDelete: (projectName: string, repoName?: string) => void;
  onContextMenu: (e: React.MouseEvent, projectName: string, repoName?: string) => void;
}

/** Returns the CSS modifier class for a worktree status string. */
function statusClass(status: string | undefined): string {
  if (!status || status === "...") return "loading";
  if (status === "clean") return "clean";
  if (status === "N/A") return "missing";
  return "dirty";
}

/** Renders a keyboard shortcut badge when `altHeld` is true; renders nothing otherwise. */
function Hint({ altHeld, isMac, k }: { altHeld: boolean; isMac: boolean; k: string }) {
  if (!altHeld) return null;
  return <span className="kbd-hint">{isMac ? `⌥${k}` : k}</span>;
}

/** Left-panel tree view listing all projects and their worktrees with inline action buttons. */
export default function ProjectTree({
  config,
  statuses,
  selectedProject,
  selectedRepo,
  expandedProjects,
  altHeld,
  isMac,
  onSelectProject,
  onSelectRepo,
  onToggleExpand,
  onNewProject,
  onAddRepo,
  onTerminal,
  onPush,
  onDiff,
  onDelete,
  onContextMenu,
}: Props) {
  return (
    <>
      <div className="tree-panel-header">
        <span className="tree-panel-title">Projects</span>
        <button className="toolbar-btn" onClick={onNewProject} title="New project (Alt+N)">
          + new<Hint altHeld={altHeld} isMac={isMac} k="N" />
        </button>
      </div>

      <div className="tree-panel-scroll">
        <div className="tree-list">
          {config.projects.length === 0 && (
            <div
              className="tree-item"
              style={{ color: "var(--text-dim)", fontStyle: "italic", fontSize: 12 }}
              onClick={onNewProject}
            >
              No projects — click to create one
            </div>
          )}

          {config.projects.map((project) => {
            const expanded = expandedProjects.has(project.name);
            const isProjectSelected = selectedProject === project.name && selectedRepo === null;

            return (
              <div key={project.name}>
                <div
                  className={`tree-item tree-item-project${isProjectSelected ? " selected" : ""}`}
                  onClick={() => { onSelectProject(project.name); onToggleExpand(project.name); }}
                  onContextMenu={(e) => { e.preventDefault(); onContextMenu(e, project.name); }}
                >
                  <span className="tree-prefix">{expanded ? "▼" : "▶"}</span>
                  <span className="tree-name">{project.name}</span>
                  <span className="tree-branch">{project.branch}</span>
                  <div className="tree-actions" onClick={(e) => e.stopPropagation()}>
                    <button
                      className="action-btn"
                      title="Add repo (Alt+A)"
                      onClick={() => onAddRepo(project.name)}
                    >
                      +repo<Hint altHeld={altHeld} isMac={isMac} k="A" />
                    </button>
                    <button
                      className="action-btn"
                      title="Open terminal (Alt+T)"
                      onClick={() => onTerminal(project.name)}
                    >
                      term<Hint altHeld={altHeld} isMac={isMac} k="T" />
                    </button>
                    <button
                      className="action-btn"
                      title="Push project (Alt+P)"
                      onClick={() => onPush(project.name)}
                    >
                      push<Hint altHeld={altHeld} isMac={isMac} k="P" />
                    </button>
                    <button
                      className="action-btn danger"
                      title="Delete project (Alt+X)"
                      onClick={() => onDelete(project.name)}
                    >
                      del<Hint altHeld={altHeld} isMac={isMac} k="X" />
                    </button>
                  </div>
                </div>

                {expanded &&
                  project.worktrees.map((wt, idx) => {
                    const isLast = idx === project.worktrees.length - 1;
                    const isSelected =
                      selectedProject === project.name && selectedRepo === wt.repo_name;
                    const statusKey = `${project.name}/${wt.repo_name}`;
                    const status = statuses[statusKey];

                    return (
                      <div
                        key={wt.repo_name}
                        className={`tree-item tree-item-worktree${isSelected ? " selected" : ""}`}
                        onClick={() => onSelectRepo(project.name, wt.repo_name)}
                        onContextMenu={(e) => { e.preventDefault(); onContextMenu(e, project.name, wt.repo_name); }}
                      >
                        <span className="tree-prefix">{isLast ? "└─" : "├─"}</span>
                        <span className="tree-repo-name">{wt.repo_name}</span>
                        <span className={`tree-status ${statusClass(status)}`}>
                          {status ?? "..."}
                        </span>
                        <div className="tree-actions" onClick={(e) => e.stopPropagation()}>
                          <button
                            className="action-btn"
                            title="Open terminal (Alt+T)"
                            onClick={() => onTerminal(project.name, wt.repo_name)}
                          >
                            term<Hint altHeld={altHeld} isMac={isMac} k="T" />
                          </button>
                          <button
                            className="action-btn"
                            title="View diff (Alt+D)"
                            onClick={() => onDiff(project.name, wt.repo_name)}
                          >
                            diff<Hint altHeld={altHeld} isMac={isMac} k="D" />
                          </button>
                          <button
                            className="action-btn"
                            title="Push (Alt+P)"
                            onClick={() => onPush(project.name, wt.repo_name)}
                          >
                            push<Hint altHeld={altHeld} isMac={isMac} k="P" />
                          </button>
                          <button
                            className="action-btn danger"
                            title="Remove worktree (Alt+X)"
                            onClick={() => onDelete(project.name, wt.repo_name)}
                          >
                            del<Hint altHeld={altHeld} isMac={isMac} k="X" />
                          </button>
                        </div>
                      </div>
                    );
                  })}
              </div>
            );
          })}
        </div>
      </div>
    </>
  );
}
