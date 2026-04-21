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
  const terminalContainerRef = useRef<HTMLDivElement>(null);
  const { statuses, refresh: refreshStatuses } = useWorktreeStatus();

  useEffect(() => {
    api.loadConfig().then((cfg) => {
      setConfig(cfg);
      setExpandedProjects(new Set(cfg.projects.map((p) => p.name)));
      if (cfg.projects.length > 0) setSelectedProject(cfg.projects[0].name);
    });
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

  const openTerminal = useCallback(() => {
    if (!config) return;
    const path = currentWorktree?.path ?? currentProject?.folder ?? null;
    if (!path) return;
    if (config.settings.use_tmux) {
      api.openExternalTerminal(path).catch((e) => setErrorMessage(String(e)));
      return;
    }
    const sessionId = currentWorktree
      ? `${selectedProject}/${selectedRepo}`
      : selectedProject!;
    setTerminalSession({ id: sessionId, path });
    setActivePanel("terminal");
  }, [config, currentProject, currentWorktree, selectedProject, selectedRepo]);

  const openDiff = useCallback(async () => {
    if (!selectedProject || !selectedRepo) {
      setErrorMessage("Select a worktree first.");
      return;
    }
    try {
      const diff = await api.getDiff(selectedProject, selectedRepo);
      setDiffContent(diff || "(no changes)");
      setActivePanel("diff");
      setErrorMessage(null);
    } catch (e) {
      setErrorMessage(String(e));
    }
  }, [selectedProject, selectedRepo]);

  const handlePush = useCallback(
    async (commitMessage?: string) => {
      setModal({ type: "none" });
      try {
        if (currentWorktree && selectedProject && selectedRepo) {
          const result = await api.pushWorktree(selectedProject, selectedRepo, commitMessage);
          setOutputLines(result.output.split("\n").filter(Boolean));
          if (result.success) { setErrorMessage(null); refreshStatuses(); }
          else setErrorMessage("Push failed — see output");
        } else if (currentProject && selectedProject) {
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
    [currentProject, currentWorktree, selectedProject, selectedRepo, refreshStatuses],
  );

  const handleSaveSettings = useCallback(
    async (settings: Settings) => {
      try {
        const updated = await api.updateSettings(settings);
        setConfig(updated);
        setModal({ type: "none" });
      } catch (e) {
        setErrorMessage(String(e));
      }
    },
    [],
  );

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
        case "n":
          setModal({ type: "addProject" });
          break;
        case "a":
          if (selectedProject) setModal({ type: "addRepo" });
          else setErrorMessage("Select a project first.");
          break;
        case "t":
          openTerminal();
          break;
        case "p":
          if (!selectedProject) { setErrorMessage("Select a project first."); break; }
          setModal({ type: "commit", scope: selectedRepo ? "repo" : "project" });
          break;
        case "d":
          openDiff();
          break;
        case "x":
          if (selectedRepo && selectedProject)
            setModal({ type: "confirmRepo", projectName: selectedProject, repoName: selectedRepo });
          else if (selectedProject)
            setModal({ type: "confirmProject", name: selectedProject });
          break;
        case "o":
          setModal({ type: "options" });
          break;
        case "h":
          setModal({ type: "help" });
          break;
        case "q":
          import("@tauri-apps/api/window").then(({ getCurrentWindow }) =>
            getCurrentWindow().close(),
          );
          break;
        case "Escape":
          if (activePanel !== "none") setActivePanel("none");
          setErrorMessage(null);
          break;
      }
    },
    [modal, selectedProject, selectedRepo, activePanel, navigateNext, navigatePrev, toggleExpand, openTerminal, openDiff],
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
        {errorMessage && <span className="app-error">{errorMessage}</span>}
      </header>

      <div className="app-body">
        <div className="panel-tree">
          <ProjectTree
            config={config}
            statuses={statuses}
            selectedProject={selectedProject}
            selectedRepo={selectedRepo}
            expandedProjects={expandedProjects}
            onSelectProject={(name) => { setSelectedProject(name); setSelectedRepo(null); setErrorMessage(null); }}
            onSelectRepo={(project, repo) => { setSelectedProject(project); setSelectedRepo(repo); setErrorMessage(null); }}
            onToggleExpand={toggleExpand}
          />
        </div>

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
            <div className="panel-empty">
              <span className="panel-hint">
                {currentWorktree
                  ? `${currentWorktree.repo_name} — ${currentProject?.branch}`
                  : currentProject
                  ? `${currentProject.name}  (${currentProject.branch})`
                  : "Press n to create a project"}
              </span>
            </div>
          )}
        </div>
      </div>

      <footer className="app-footer">
        <span>[n] new</span>
        <span>[a] add repo</span>
        <span>[t] terminal</span>
        <span>[p] push</span>
        <span>[d] diff</span>
        <span>[x] delete</span>
        <span>[o] options</span>
        <span>[h] help</span>
        <span>[q] quit</span>
      </footer>

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
    </div>
  );
}
