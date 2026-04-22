import { useCallback, useEffect, useRef, useState } from "react";
import { Config, Settings } from "./api/types";
import * as api from "./api/tauri";
import ProjectTree from "./components/ProjectTree";
import TerminalPane from "./components/TerminalPane";
import OutputPane from "./components/OutputPane";
import AddProjectModal from "./components/AddProjectModal";
import AddRepoModal from "./components/AddRepoModal";
import CommitMessageModal from "./components/CommitMessageModal";
import ConfirmDeleteModal from "./components/ConfirmDeleteModal";
import HelpModal from "./components/HelpModal";
import OptionsModal from "./components/OptionsModal";
import { useWorktreeStatus } from "./hooks/useWorktreeStatus";
import ContextMenu, { MenuItem } from "./components/ContextMenu";

const isMac = /Mac|iPhone|iPad/.test(navigator.platform);

type Panel = "none" | "terminal" | "output" | "diff";

type Modal =
  | { type: "none" }
  | { type: "addProject" }
  | { type: "addRepo" }
  | { type: "commit"; scope: "project" | "repo" }
  | { type: "confirmProject"; name: string }
  | { type: "confirmRepo"; projectName: string; repoName: string }
  | { type: "help" }
  | { type: "options" };

type TreeItem =
  | { type: "project"; key: string; projectName: string }
  | { type: "worktree"; key: string; projectName: string; repoName: string };

function buildTreeItems(config: Config, expanded: Set<string>): TreeItem[] {
  const items: TreeItem[] = [];
  for (const p of config.projects) {
    items.push({ type: "project", key: `p:${p.name}`, projectName: p.name });
    if (expanded.has(p.name)) {
      for (const wt of p.worktrees) {
        items.push({
          type: "worktree",
          key: `wt:${p.name}:${wt.repo_name}`,
          projectName: p.name,
          repoName: wt.repo_name,
        });
      }
    }
  }
  return items;
}

/** Renders a platform-appropriate keyboard shortcut badge (e.g. `⌥N` on Mac, `Alt+N` elsewhere). */
function KbdHint({ k }: { k: string }) {
  return <span className="kbd-hint">{isMac ? `⌥${k}` : `Alt+${k}`}</span>;
}

/** Root application component. Owns all global state (config, selection, active panel, modals). */
export default function App() {
  const [config, setConfig] = useState<Config | null>(null);
  const [selectedProject, setSelectedProject] = useState<string | null>(null);
  const [selectedRepo, setSelectedRepo] = useState<string | null>(null);
  const [expandedProjects, setExpandedProjects] = useState<Set<string>>(new Set());
  const [activePanel, setActivePanel] = useState<Panel>("none");
  const [terminalSession, setTerminalSession] = useState<{ id: string; path: string } | null>(null);
  const [outputLines, setOutputLines] = useState<string[]>([]);
  const [diffContent, setDiffContent] = useState("");
  const [modal, setModal] = useState<Modal>({ type: "none" });
  const [errorMessage, setErrorMessage] = useState<string | null>(null);
  const [altHeld, setAltHeld] = useState(false);
  const [contextMenu, setContextMenu] = useState<{ x: number; y: number; items: MenuItem[] } | null>(null);
  const projectMenuBtnRef = useRef<HTMLButtonElement>(null);
  const terminalContainerRef = useRef<HTMLDivElement>(null);
  const [treePanelWidth, setTreePanelWidth] = useState(340);
  const isDragging = useRef(false);
  const dragStartX = useRef(0);
  const dragStartWidth = useRef(0);
  const { statuses, refresh: refreshStatuses } = useWorktreeStatus();

  useEffect(() => {
    api.loadConfig().then((cfg) => {
      setConfig(cfg);
      setExpandedProjects(new Set(cfg.projects.map((p) => p.name)));
      if (cfg.projects.length > 0) setSelectedProject(cfg.projects[0].name);
    });
  }, []);

  useEffect(() => {
    const down = (e: KeyboardEvent) => { if (e.key === "Alt") setAltHeld(true); };
    const up = (e: KeyboardEvent) => { if (e.key === "Alt") setAltHeld(false); };
    const blur = () => setAltHeld(false);
    window.addEventListener("keydown", down);
    window.addEventListener("keyup", up);
    window.addEventListener("blur", blur);
    return () => {
      window.removeEventListener("keydown", down);
      window.removeEventListener("keyup", up);
      window.removeEventListener("blur", blur);
    };
  }, []);

  const currentProject = config?.projects.find((p) => p.name === selectedProject) ?? null;
  const currentWorktree = currentProject?.worktrees.find((w) => w.repo_name === selectedRepo) ?? null;

  const selectedKey = selectedRepo
    ? `wt:${selectedProject}:${selectedRepo}`
    : selectedProject
    ? `p:${selectedProject}`
    : null;

  const treeItems = config ? buildTreeItems(config, expandedProjects) : [];

  const selectItem = useCallback((item: TreeItem) => {
    setSelectedProject(item.projectName);
    setSelectedRepo(item.type === "worktree" ? item.repoName : null);
    setErrorMessage(null);
  }, []);

  const navigateNext = useCallback(() => {
    if (!treeItems.length) return;
    const idx = treeItems.findIndex((i) => i.key === selectedKey);
    selectItem(treeItems[idx < 0 ? 0 : (idx + 1) % treeItems.length]);
  }, [treeItems, selectedKey, selectItem]);

  const navigatePrev = useCallback(() => {
    if (!treeItems.length) return;
    const idx = treeItems.findIndex((i) => i.key === selectedKey);
    selectItem(treeItems[idx <= 0 ? treeItems.length - 1 : idx - 1]);
  }, [treeItems, selectedKey, selectItem]);

  const toggleExpand = useCallback((name: string) => {
    setExpandedProjects((prev) => {
      const next = new Set(prev);
      if (next.has(name)) next.delete(name);
      else next.add(name);
      return next;
    });
  }, []);

  const openTerminalFor = useCallback(
    (projectName: string, repoName?: string) => {
      if (!config) return;
      const project = config.projects.find((p) => p.name === projectName);
      const wt = repoName ? project?.worktrees.find((w) => w.repo_name === repoName) : null;
      const path = wt?.path ?? project?.folder ?? null;
      if (!path) return;
      setSelectedProject(projectName);
      setSelectedRepo(repoName ?? null);
      if (config.settings.use_tmux) {
        api.openExternalTerminal(path).catch((e) => setErrorMessage(String(e)));
        return;
      }
      const sessionId = repoName ? `${projectName}/${repoName}` : projectName;
      setTerminalSession({ id: sessionId, path });
      setActivePanel("terminal");
    },
    [config],
  );

  const pushFor = useCallback((projectName: string, repoName?: string) => {
    setSelectedProject(projectName);
    setSelectedRepo(repoName ?? null);
    setModal({ type: "commit", scope: repoName ? "repo" : "project" });
  }, []);

  const diffFor = useCallback(async (projectName: string, repoName: string) => {
    setSelectedProject(projectName);
    setSelectedRepo(repoName);
    try {
      const diff = await api.getDiff(projectName, repoName);
      setDiffContent(diff || "(no changes)");
      setActivePanel("diff");
      setErrorMessage(null);
    } catch (e) {
      setErrorMessage(String(e));
    }
  }, []);

  const deleteFor = useCallback((projectName: string, repoName?: string) => {
    setSelectedProject(projectName);
    setSelectedRepo(repoName ?? null);
    if (repoName) {
      setModal({ type: "confirmRepo", projectName, repoName });
    } else {
      setModal({ type: "confirmProject", name: projectName });
    }
  }, []);

  const addRepoFor = useCallback((projectName: string) => {
    setSelectedProject(projectName);
    setModal({ type: "addRepo" });
  }, []);

  const openInVscode = useCallback(async (projectName: string) => {
    try {
      await api.openInVscode(projectName);
    } catch (e) {
      setErrorMessage(String(e));
    }
  }, []);

  const buildProjectMenu = useCallback(
    (projectName: string, repoName?: string): MenuItem[] => {
      const items: MenuItem[] = [
        {
          type: "item",
          label: "Open in VS Code",
          icon: "⬡",
          onClick: () => openInVscode(projectName),
        },
        { type: "separator" },
        {
          type: "item",
          label: "Open Terminal",
          onClick: () => openTerminalFor(projectName, repoName),
        },
      ];
      if (repoName) {
        items.push({
          type: "item",
          label: "View Diff",
          onClick: () => diffFor(projectName, repoName),
        });
      }
      items.push({
        type: "item",
        label: repoName ? "Push Worktree" : "Push Project",
        onClick: () => pushFor(projectName, repoName),
      });
      if (!repoName) {
        items.push({
          type: "item",
          label: "Add Repo",
          onClick: () => addRepoFor(projectName),
        });
      }
      items.push(
        { type: "separator" },
        {
          type: "item",
          label: repoName ? "Remove Worktree" : "Delete Project",
          danger: true,
          onClick: () => deleteFor(projectName, repoName),
        },
      );
      return items;
    },
    [openInVscode, openTerminalFor, diffFor, pushFor, addRepoFor, deleteFor],
  );

  const openContextMenu = useCallback(
    (e: React.MouseEvent, projectName: string, repoName?: string) => {
      setContextMenu({ x: e.clientX, y: e.clientY, items: buildProjectMenu(projectName, repoName) });
    },
    [buildProjectMenu],
  );

  const startSplitDrag = useCallback((e: React.MouseEvent) => {
    e.preventDefault();
    isDragging.current = true;
    dragStartX.current = e.clientX;
    dragStartWidth.current = treePanelWidth;
    const onMove = (ev: MouseEvent) => {
      if (!isDragging.current) return;
      const delta = ev.clientX - dragStartX.current;
      const next = Math.max(200, Math.min(window.innerWidth * 0.5, dragStartWidth.current + delta));
      setTreePanelWidth(next);
    };
    const onUp = () => {
      isDragging.current = false;
      document.removeEventListener("mousemove", onMove);
      document.removeEventListener("mouseup", onUp);
    };
    document.addEventListener("mousemove", onMove);
    document.addEventListener("mouseup", onUp);
  }, [treePanelWidth]);

  const openProjectMenuFromBtn = useCallback(() => {
    if (!projectMenuBtnRef.current || !selectedProject) return;
    const rect = projectMenuBtnRef.current.getBoundingClientRect();
    setContextMenu({
      x: rect.left,
      y: rect.bottom + 4,
      items: buildProjectMenu(selectedProject, selectedRepo ?? undefined),
    });
  }, [selectedProject, selectedRepo, buildProjectMenu]);

  const handlePush = useCallback(
    async (commitMessage?: string) => {
      setModal({ type: "none" });
      try {
        if (selectedRepo && selectedProject) {
          const result = await api.pushWorktree(selectedProject, selectedRepo, commitMessage);
          setOutputLines(result.output.split("\n").filter(Boolean));
          if (result.success) { setErrorMessage(null); refreshStatuses(); }
          else setErrorMessage("Push failed — see output");
        } else if (selectedProject) {
          const results = await api.pushProject(selectedProject, commitMessage);
          setOutputLines(results.map((r) => `${r.success ? "✓" : "✗"} [${r.repo_name}]  ${r.detail}`));
          const allOk = results.every((r) => r.success);
          if (allOk) { setErrorMessage(null); refreshStatuses(); }
          else setErrorMessage("Some pushes failed — see output");
        }
        setActivePanel("output");
      } catch (e) {
        setErrorMessage(String(e));
      }
    },
    [selectedProject, selectedRepo, refreshStatuses],
  );

  const handleSaveSettings = useCallback(async (settings: Settings) => {
    try {
      const updated = await api.updateSettings(settings);
      setConfig(updated);
      setModal({ type: "none" });
    } catch (e) {
      setErrorMessage(String(e));
    }
  }, []);

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if (modal.type !== "none") return;
      if (terminalContainerRef.current?.contains(document.activeElement)) return;

      switch (e.key) {
        case "ArrowDown":
          e.preventDefault();
          navigateNext();
          break;
        case "ArrowUp":
          e.preventDefault();
          navigatePrev();
          break;
        case "Enter":
          if (!selectedRepo && selectedProject) toggleExpand(selectedProject);
          break;
        case "Escape":
          if (activePanel !== "none") setActivePanel("none");
          setErrorMessage(null);
          break;
        case "n":
          if (!e.altKey) break;
          setModal({ type: "addProject" });
          break;
        case "a":
          if (!e.altKey) break;
          if (selectedProject) setModal({ type: "addRepo" });
          else setErrorMessage("Select a project first.");
          break;
        case "t":
          if (!e.altKey) break;
          if (selectedProject) openTerminalFor(selectedProject, selectedRepo ?? undefined);
          break;
        case "p":
          if (!e.altKey) break;
          if (!selectedProject) { setErrorMessage("Select a project first."); break; }
          pushFor(selectedProject, selectedRepo ?? undefined);
          break;
        case "d":
          if (!e.altKey) break;
          if (selectedProject && selectedRepo) diffFor(selectedProject, selectedRepo);
          else setErrorMessage("Select a worktree first.");
          break;
        case "x":
          if (!e.altKey) break;
          if (selectedProject) deleteFor(selectedProject, selectedRepo ?? undefined);
          break;
        case "o":
          if (!e.altKey) break;
          setModal({ type: "options" });
          break;
        case "h":
          if (!e.altKey) break;
          setModal({ type: "help" });
          break;
        case "q":
          if (!e.altKey) break;
          import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
            getCurrentWindow().close(),
          );
          break;
      }
    },
    [modal, selectedProject, selectedRepo, activePanel, navigateNext, navigatePrev, toggleExpand, openTerminalFor, pushFor, diffFor, deleteFor],
  );

  useEffect(() => {
    document.addEventListener("keydown", handleKeyDown);
    return () => document.removeEventListener("keydown", handleKeyDown);
  }, [handleKeyDown]);

  if (!config) {
    return <div className="app-loading">Loading…</div>;
  }

  return (
    <div className="app">
      <header className="app-header">
        <span className="app-title">workman</span>
        {errorMessage
          ? <span className="app-error">{errorMessage}</span>
          : <span className="header-spacer" />
        }
        <div className="header-toolbar">
          {selectedProject && (
            <button
              ref={projectMenuBtnRef}
              className="toolbar-btn toolbar-btn-accent"
              onClick={openProjectMenuFromBtn}
              title="Project menu"
            >
              Project ▾
            </button>
          )}
          <button className="toolbar-btn" onClick={() => setModal({ type: "options" })} title="Options (Alt+O)">
            options{altHeld && <KbdHint k="O" />}
          </button>
          <button className="toolbar-btn" onClick={() => setModal({ type: "help" })} title="Help (Alt+H)">
            help{altHeld && <KbdHint k="H" />}
          </button>
        </div>
      </header>

      <div className="app-body">
        <div className="panel-tree" style={{ width: treePanelWidth, flexShrink: 0 }}>
          <ProjectTree
            config={config}
            statuses={statuses}
            selectedProject={selectedProject}
            selectedRepo={selectedRepo}
            expandedProjects={expandedProjects}
            altHeld={altHeld}
            isMac={isMac}
            onSelectProject={(name) => { setSelectedProject(name); setSelectedRepo(null); setErrorMessage(null); }}
            onSelectRepo={(project, repo) => { setSelectedProject(project); setSelectedRepo(repo); setErrorMessage(null); }}
            onToggleExpand={toggleExpand}
            onNewProject={() => setModal({ type: "addProject" })}
            onAddRepo={addRepoFor}
            onTerminal={openTerminalFor}
            onPush={pushFor}
            onDiff={diffFor}
            onDelete={deleteFor}
            onContextMenu={openContextMenu}
          />
        </div>

        <div className="split-bar" onMouseDown={startSplitDrag} />

        <div className="panel-content">
          {activePanel === "terminal" && terminalSession && (
            <div ref={terminalContainerRef} className="terminal-wrapper">
              <TerminalPane
                key={terminalSession.id}
                sessionId={terminalSession.id}
                workingDir={terminalSession.path}
                onClose={() => setActivePanel("none")}
              />
            </div>
          )}
          {(activePanel === "output" || activePanel === "diff") && (
            <OutputPane
              lines={activePanel === "diff" ? diffContent.split("\n") : outputLines}
              isDiff={activePanel === "diff"}
              onClose={() => setActivePanel("none")}
            />
          )}
          {activePanel === "none" && (
            <div className="action-panel">
              {currentWorktree ? (
                <>
                  <div className="action-panel-info">
                    <div className="action-panel-name">{currentWorktree.repo_name}</div>
                    <div className="action-panel-branch">{currentProject?.branch}</div>
                    <div className="action-panel-sub">{currentProject?.name}</div>
                  </div>
                  <div className="action-panel-grid">
                    <button className="panel-action-btn" onClick={() => openTerminalFor(selectedProject!, selectedRepo!)}>
                      &gt;_ terminal{altHeld && <KbdHint k="T" />}
                    </button>
                    <button className="panel-action-btn" onClick={() => diffFor(selectedProject!, selectedRepo!)}>
                      ≠ diff{altHeld && <KbdHint k="D" />}
                    </button>
                    <button className="panel-action-btn" onClick={() => pushFor(selectedProject!, selectedRepo!)}>
                      ↑ push{altHeld && <KbdHint k="P" />}
                    </button>
                    <button className="panel-action-btn danger" onClick={() => deleteFor(selectedProject!, selectedRepo!)}>
                      × remove{altHeld && <KbdHint k="X" />}
                    </button>
                  </div>
                </>
              ) : currentProject ? (
                <>
                  <div className="action-panel-info">
                    <div className="action-panel-name">{currentProject.name}</div>
                    <div className="action-panel-branch">{currentProject.branch}</div>
                  </div>
                  <div className="action-panel-grid">
                    <button className="panel-action-btn" onClick={() => openTerminalFor(selectedProject!)}>
                      &gt;_ terminal{altHeld && <KbdHint k="T" />}
                    </button>
                    <button className="panel-action-btn" onClick={() => pushFor(selectedProject!)}>
                      ↑ push{altHeld && <KbdHint k="P" />}
                    </button>
                    <button className="panel-action-btn" onClick={() => addRepoFor(selectedProject!)}>
                      + add repo{altHeld && <KbdHint k="A" />}
                    </button>
                    <button className="panel-action-btn danger" onClick={() => deleteFor(selectedProject!)}>
                      × delete{altHeld && <KbdHint k="X" />}
                    </button>
                  </div>
                </>
              ) : (
                <div className="panel-empty">
                  <span className="panel-hint">Select or create a project</span>
                </div>
              )}
            </div>
          )}
        </div>
      </div>

      {modal.type === "addProject" && (
        <AddProjectModal
          onConfirm={async (name) => {
            try {
              const cfg = await api.createProject(name);
              setConfig(cfg);
              setSelectedProject(name);
              setSelectedRepo(null);
              setExpandedProjects((prev) => new Set([...prev, name]));
              setModal({ type: "addRepo" });
            } catch (e) {
              setErrorMessage(String(e));
              setModal({ type: "none" });
            }
          }}
          onCancel={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "addRepo" && selectedProject && (
        <AddRepoModal
          projectName={selectedProject}
          knownRepos={config.repos.map((r) => r.path)}
          onAdd={async (repoPath) => {
            try {
              const cfg = await api.addRepoToProject(selectedProject, repoPath);
              setConfig(cfg);
              refreshStatuses();
              setErrorMessage(null);
            } catch (e) {
              throw e;
            }
          }}
          onClose={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "commit" && (
        <CommitMessageModal
          scope={
            modal.scope === "repo"
              ? `${selectedProject}/${selectedRepo}`
              : selectedProject!
          }
          onConfirm={handlePush}
          onCancel={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "confirmProject" && (
        <ConfirmDeleteModal
          message={`Remove project "${modal.name}" and all its worktrees?`}
          onConfirm={async () => {
            try {
              const cfg = await api.removeProject(modal.name);
              setConfig(cfg);
              setSelectedProject(cfg.projects[0]?.name ?? null);
              setSelectedRepo(null);
            } catch (e) {
              setErrorMessage(String(e));
            }
            setModal({ type: "none" });
          }}
          onCancel={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "confirmRepo" && (
        <ConfirmDeleteModal
          message={`Remove worktree "${modal.repoName}" from project "${modal.projectName}"?`}
          onConfirm={async () => {
            try {
              const cfg = await api.removeWorktree(modal.projectName, modal.repoName);
              setConfig(cfg);
              setSelectedRepo(null);
              refreshStatuses();
            } catch (e) {
              setErrorMessage(String(e));
            }
            setModal({ type: "none" });
          }}
          onCancel={() => setModal({ type: "none" })}
        />
      )}

      {modal.type === "help" && (
        <HelpModal onClose={() => setModal({ type: "none" })} />
      )}

      {modal.type === "options" && (
        <OptionsModal
          settings={config.settings}
          onSave={handleSaveSettings}
          onClose={() => setModal({ type: "none" })}
        />
      )}

      {contextMenu && (
        <ContextMenu
          x={contextMenu.x}
          y={contextMenu.y}
          items={contextMenu.items}
          onClose={() => setContextMenu(null)}
        />
      )}
    </div>
  );
}
