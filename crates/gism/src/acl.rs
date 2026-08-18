use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct AclFile {
    #[serde(rename = "repo")]
    pub repos: Vec<RepoAcl>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RepoAcl {
    pub name: String,
    #[serde(default)]
    pub read: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
}

impl AclFile {
    pub fn path() -> Result<PathBuf> {
        let home = std::env::var("HOME")?;
        Ok(PathBuf::from(home).join(".config/gism/acl.toml"))
    }

    pub fn load() -> Result<Self> {
        let path = Self::path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(path)?;
        Ok(toml::from_str(&content)?)
    }

    pub fn save(&self) -> Result<()> {
        let path = Self::path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, toml::to_string_pretty(self)?)?;
        Ok(())
    }

    pub fn grant(&mut self, repo: &str, user: &str, write: bool) {
        if let Some(r) = self.repos.iter_mut().find(|r| r.name == repo) {
            if write && !r.write.contains(&user.to_string()) {
                r.write.push(user.to_string());
            }
            if !r.read.contains(&user.to_string()) {
                r.read.push(user.to_string());
            }
        } else {
            self.repos.push(RepoAcl {
                name: repo.to_string(),
                read: vec![user.to_string()],
                write: if write {
                    vec![user.to_string()]
                } else {
                    vec![]
                },
            });
        }
    }

    pub fn revoke(&mut self, repo: &str, user: &str) {
        if let Some(r) = self.repos.iter_mut().find(|r| r.name == repo) {
            r.read.retain(|u| u != user);
            r.write.retain(|u| u != user);
        }
    }

    pub fn check(&self, repo: &str, user: &str, write: bool) -> bool {
        self.repos
            .iter()
            .find(|r| r.name == repo)
            .map(|r| {
                if write {
                    r.write.contains(&user.to_string())
                } else {
                    r.read.contains(&user.to_string()) || r.write.contains(&user.to_string())
                }
            })
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mock_acl() -> AclFile {
        let mut acl = AclFile::default();
        acl.grant("alice/repo", "bob", false);
        acl.grant("alice/repo", "charlie", true);
        acl
    }

    #[test]
    fn test_grant_and_check() {
        let acl = mock_acl();
        assert!(acl.check("alice/repo", "bob", false));
        assert!(!acl.check("alice/repo", "bob", true));

        assert!(acl.check("alice/repo", "charlie", true));
        assert!(acl.check("alice/repo", "charlie", false));
    }

    #[test]
    fn test_revoke() {
        let mut acl = mock_acl();
        acl.revoke("alice/repo", "bob");
        assert!(!acl.check("alice/repo", "bob", false));
        assert!(!acl.check("alice/repo", "bob", true));
    }

    #[test]
    fn test_non_existent_repo() {
        let acl = mock_acl();
        assert!(!acl.check("alice/secret", "bob", false));
        assert!(!acl.check("alice/secret", "bob", true));
    }
}
