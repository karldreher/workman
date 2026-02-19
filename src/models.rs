use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Worktree {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Project {
    pub name: String,
    pub path: PathBuf,
    pub worktrees: Vec<Worktree>,
}

impl Project {
    pub fn remove_worktree(&mut self, w_idx: usize) -> Result<std::process::Output> {
        let wt = &self.worktrees[w_idx];
        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .arg("worktree")
            .arg("remove")
            .arg(&wt.name)
            .output()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn add_worktree(&mut self, _name: &str, path: PathBuf, branch: &str) -> Result<std::process::Output> {
        // Handle .workman/ directory and .gitignore
        let workman_dir = self.path.join(".workman");
        if !workman_dir.exists() {
            let _ = fs::create_dir_all(&workman_dir);
        }

        let gitignore_path = self.path.join(".gitignore");
        let mut needs_append = true;
        if let Ok(content) = fs::read_to_string(&gitignore_path) {
            if content.lines().any(|l| l.trim() == ".workman/" || l.trim() == ".workman") {
                needs_append = false;
            }
        }
        if needs_append {
            use std::io::Write;
            if let Ok(mut file) = fs::OpenOptions::new()
                .append(true)
                .create(true)
                .open(&gitignore_path) 
            {
                let _ = writeln!(file, "\n# workman worktrees\n.workman/");
            }
        }

        // 1. Validate branch name format
        let valid_format = std::process::Command::new("git")
            .arg("-C").arg(&self.path)
            .arg("check-ref-format")
            .arg("--normalize")
            .arg(format!("refs/heads/{}", branch))
            .output()?;

        if !valid_format.status.success() {
            return Ok(valid_format);
        }
        
        // 2. Check if branch exists
        let branch_exists = std::process::Command::new("git")
            .arg("-C").arg(&self.path)
            .arg("show-ref")
            .arg("--verify")
            .arg(format!("refs/heads/{}", branch))
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        let mut cmd = std::process::Command::new("git");
        cmd.arg("-C").arg(&self.path).arg("worktree").arg("add");
        
        if !branch_exists {
            cmd.arg("-b").arg(branch).arg(&path);
        } else {
            cmd.arg(&path).arg(branch);
        }
        
        cmd.output().map_err(|e| anyhow::anyhow!(e))
    }
}

impl Worktree {
    pub fn push(&self) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .arg("push")
            .output()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_diff(&self) -> Result<std::process::Output> {
        std::process::Command::new("git")
            .arg("-C")
            .arg(&self.path)
            .arg("diff")
            .output()
            .map_err(|e| anyhow::anyhow!(e))
    }

    pub fn get_status(&self) -> String {
        if !self.path.exists() {
            return "N/A".to_string();
        }

        let git_dir_arg = format!("--git-dir={}/.git", self.path.display());
        let work_tree_arg = format!("--work-tree={}", self.path.display());

        let diff_numstat_output = std::process::Command::new("git")
            .arg(&git_dir_arg)
            .arg(&work_tree_arg)
            .arg("diff")
            .arg("--numstat")
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
            .arg(&git_dir_arg)
            .arg(&work_tree_arg)
            .arg("status")
            .arg("--porcelain=v1")
            .output();

        if let Ok(output) = untracked_status_output {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let untracked_count = stdout.lines().filter(|line| line.starts_with("??")).count();
            if untracked_count > 0 {
                status_indicators.push(format!("U:{}", untracked_count));
            }
        }

        let unpushed_output = std::process::Command::new("git")
            .arg(&git_dir_arg)
            .arg(&work_tree_arg)
            .arg("cherry")
            .arg("-v")
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

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct Config {
    pub projects: Vec<Project>,
}

impl Config {
    pub fn get_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".workman.config")
    }

    pub fn load() -> Self {
        let path = Self::get_path();
        if path.exists() {
            let content = fs::read_to_string(path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::get_path();
        let content = serde_json::to_string_pretty(self)?;
        fs::write(path, content)?;
        Ok(())
    }

    pub fn validate_project_path(path: &PathBuf) -> Result<()> {
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
        config.projects.push(Project {
            name: "test".to_string(),
            path: PathBuf::from("/tmp/test"),
            worktrees: vec![Worktree {
                name: "wt1".to_string(),
                path: PathBuf::from("/tmp/test/wt1"),
            }],
        });

        let json = serde_json::to_string(&config).unwrap();
        let decoded: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.projects.len(), 1);
        assert_eq!(decoded.projects[0].name, "test");
        assert_eq!(decoded.projects[0].worktrees.len(), 1);
        assert_eq!(decoded.projects[0].worktrees[0].name, "wt1");
    }

    #[test]
    fn test_project_worktree_structs() {
        let wt = Worktree {
            name: "wt".to_string(),
            path: PathBuf::from("/path/to/wt"),
        };
        let project = Project {
            name: "proj".to_string(),
            path: PathBuf::from("/path/to/proj"),
            worktrees: vec![wt.clone()],
        };

        assert_eq!(project.worktrees[0].name, wt.name);
        assert_eq!(project.worktrees[0].path, wt.path);
    }

    #[test]
    fn test_validate_project_path() {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().to_path_buf();
        
        // Should fail if no .git
        assert!(Config::validate_project_path(&path).is_err());

        // Should pass if .git exists
        fs::create_dir(path.join(".git")).unwrap();
        assert!(Config::validate_project_path(&path).is_ok());

        // Should fail if path does not exist
        let non_existent = PathBuf::from("/nonexistent/path/for/workman/test");
        assert!(Config::validate_project_path(&non_existent).is_err());
    }
}
