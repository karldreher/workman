use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

/// A registered git repository in the global pool.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Repo {
    pub name: String,
    pub path: PathBuf,
}

impl Repo {
    /// Creates a git worktree for this repo on the given branch at the given destination.
    /// The caller is responsible for creating the parent directory of `dest`.
    /// Returns the git command output and the destination path.
    pub fn add_worktree(&self, branch: &str, dest: PathBuf) -> Result<(std::process::Output, PathBuf)> {
        let valid_format = std::process::Command::new("git")
            .arg("-C").arg(&self.path)
            .arg("check-ref-format").arg("--normalize")
            .arg(format!("refs/heads/{}", branch))
            .output()?;

        if !valid_format.status.success() {
            return Ok((valid_format, PathBuf::new()));
        }

        let branch_exists = std::process::Command::new("git")
            .arg("-C").arg(&self.path)
            .arg("show-ref").arg("--verify")
            .arg(format!("refs/heads/{}", branch))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let mut cmd = std::process::Command::new("git");
        cmd.arg("-C").arg(&self.path).arg("worktree").arg("add");
        if !branch_exists {
            cmd.arg("-b").arg(branch).arg(&dest);
        } else {
            cmd.arg(&dest).arg(branch);
        }

        let output = cmd.output().map_err(|e| anyhow::anyhow!(e))?;
        Ok((output, dest))
    }

    /// Removes a worktree from this repo by path.
    pub fn remove_worktree(&self, wt_path: &PathBuf) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .arg("-C").arg(&self.path)
            .arg("worktree").arg("remove").arg("--force").arg(wt_path)
            .output().map_err(|e| anyhow::anyhow!(e))
    }
}

/// A worktree within a Project, associated with a specific Repo by name.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProjectWorktree {
    pub repo_name: String,
    pub path: PathBuf,
}

impl ProjectWorktree {
    pub fn push(&self, commit_message: Option<String>) -> Result<(std::process::Output, std::process::Output, std::process::Output)> {
        let add_output = std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("add").arg("-A")
            .output().map_err(|e| anyhow::anyhow!(e))?;

        let message = commit_message.unwrap_or_else(|| "workman: auto-commit".to_string());
        let commit_output = std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("commit").arg("-m").arg(message)
            .output().map_err(|e| anyhow::anyhow!(e))?;

        let push_output = std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("push")
            .output().map_err(|e| anyhow::anyhow!(e))?;

        Ok((add_output, commit_output, push_output))
    }

    pub fn get_diff(&self) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("diff")
            .output().map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_status(&self) -> String {
        if !self.path.exists() {
            return "N/A".to_string();
        }

        let diff_numstat_output = std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("diff").arg("--numstat")
            .output();

        let mut total_insertions = 0;
        let mut total_deletions = 0;
        let mut status_indicators = Vec::new();

        if let Ok(output) = diff_numstat_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() == 3 {
                    if parts[0] != "-" {
                        if let Ok(added) = parts[0].parse::<i32>() {
                            total_insertions += added;
                        }
                    }
                    if parts[1] != "-" {
                        if let Ok(deleted) = parts[1].parse::<i32>() {
                            total_deletions += deleted;
                        }
                    }
                }
            }
        }
        status_indicators.push(format!("{}/-{}", total_insertions, total_deletions));

        let untracked_status_output = std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("status").arg("--porcelain=v1")
            .output();

        if let Ok(output) = untracked_status_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let untracked_count = stdout.lines().filter(|line| line.starts_with("??")).count();
            if untracked_count > 0 {
                status_indicators.push(format!("U:{}", untracked_count));
            }
        }

        let unpushed_output = std::process::Command::new("git")
            .arg("-C").arg(&self.path).arg("cherry").arg("-v")
            .output();

        if let Ok(output) = unpushed_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let unpushed_count = stdout.lines().count();
            if unpushed_count > 0 {
                status_indicators.push(format!("↑{}", unpushed_count));
            }
        }

        if status_indicators.len() == 1 && status_indicators[0] == "0/-0" {
            "clean".to_string()
        } else {
            status_indicators.join(" ")
        }
    }
}

/// A named project grouping worktrees across multiple repos, all on the same branch.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub branch: String,
    pub worktrees: Vec<ProjectWorktree>,
    pub folder: PathBuf,
}

impl Project {
    /// Returns the base directory for all project folders.
    pub fn make_folder_path(project_name: &str) -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".workman")
            .join("projects")
            .join(project_name)
    }

    /// Creates the project folder.
    pub fn create_folder(&self) -> Result<()> {
        fs::create_dir_all(&self.folder)?;
        Ok(())
    }

    /// Removes the project folder and all worktree checkouts inside it.
    pub fn remove_folder(&self) -> Result<()> {
        if self.folder.exists() {
            fs::remove_dir_all(&self.folder)?;
        }
        Ok(())
    }
}

/// Global application settings.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Settings {
    /// Open the system terminal app instead of the built-in xterm pane.
    #[serde(default, alias = "use_tmux")]
    pub use_external_terminal: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { use_external_terminal: false }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    #[serde(default)]
    pub repos: Vec<Repo>,
    #[serde(default)]
    pub projects: Vec<Project>,
    #[serde(default)]
    pub settings: Settings,
}

impl Config {
    pub fn get_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".workman.config")
    }

    /// Loads config from disk, migrating from the legacy format if needed.
    /// Returns the config and an optional migration notice.
    pub fn load() -> (Self, Option<String>) {
        let path = Self::get_path();
        if !path.exists() {
            return (Self::default(), None);
        }
        let content = fs::read_to_string(&path).unwrap_or_default();

        let raw: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return (Self::default(), None),
        };

        if raw.get("repos").is_some() {
            let config = serde_json::from_value::<Config>(raw).unwrap_or_default();
            return (config, None);
        }

        // Migrate from legacy format
        #[derive(Deserialize)]
        struct LegacyProject {
            name: String,
            path: PathBuf,
        }
        #[derive(Deserialize)]
        struct LegacyConfig {
            projects: Vec<LegacyProject>,
        }

        if let Ok(legacy) = serde_json::from_str::<LegacyConfig>(&content) {
            let repos: Vec<Repo> = legacy.projects.into_iter().map(|p| Repo {
                name: p.name,
                path: p.path,
            }).collect();
            let new_config = Config {
                repos,
                projects: Vec::new(),
                settings: Settings::default(),
            };
            let _ = new_config.save();
            return (new_config, Some(
                "Config migrated to new format. Repos preserved — create your first project with 'n'.".to_string()
            ));
        }

        (Self::default(), None)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    /// Validates that a path is a valid, accessible git repository.
    pub fn validate_repo_path(path: &PathBuf) -> Result<()> {
        if !path.exists() {
            return Err(anyhow::anyhow!("Path does not exist: {:?}", path));
        }
        let absolute_path = fs::canonicalize(path)?;
        if !absolute_path.is_dir() {
            return Err(anyhow::anyhow!("Path is not a directory: {:?}", absolute_path));
        }
        let git_dir = absolute_path.join(".git");
        if !git_dir.exists() {
            return Err(anyhow::anyhow!("Path is not a Git repository (no .git folder): {:?}", absolute_path));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_serialization() {
        let mut config = Config::default();
        config.repos.push(Repo {
            name: "myrepo".to_string(),
            path: PathBuf::from("/tmp/myrepo"),
        });
        config.projects.push(Project {
            name: "my-feature".to_string(),
            branch: "feat/my-feature".to_string(),
            folder: PathBuf::from("/tmp/.workman/projects/my-feature"),
            worktrees: vec![ProjectWorktree {
                repo_name: "myrepo".to_string(),
                path: PathBuf::from("/tmp/.workman/projects/my-feature/myrepo"),
            }],
        });

        let json = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.repos.len(), 1);
        assert_eq!(decoded.repos[0].name, "myrepo");
        assert_eq!(decoded.projects.len(), 1);
        assert_eq!(decoded.projects[0].branch, "feat/my-feature");
        assert_eq!(decoded.projects[0].worktrees.len(), 1);
        assert_eq!(decoded.projects[0].worktrees[0].repo_name, "myrepo");
    }

    #[test]
    fn test_validate_repo_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();

        assert!(Config::validate_repo_path(&path).is_err());

        fs::create_dir(path.join(".git")).unwrap();
        assert!(Config::validate_repo_path(&path).is_ok());

        let non_existent = PathBuf::from("/nonexistent/path/for/workman/test");
        assert!(Config::validate_repo_path(&non_existent).is_err());
    }

    #[test]
    fn test_settings_default() {
        let s = Settings::default();
        assert!(!s.use_external_terminal);
    }
}
