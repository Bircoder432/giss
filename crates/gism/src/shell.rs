use crate::acl::AclFile;
use anyhow::{Result, bail};
use std::process::Command;

pub fn run(user: &str) -> Result<()> {
    let original_cmd = std::env::var("SSH_ORIGINAL_COMMAND").unwrap_or_default();

    if original_cmd.is_empty() {
        println!(
            "Hi {}! You've successfully authenticated, but gism does not provide shell access.",
            user
        );
        println!("Run 'giss list' on your local machine to see repositories.");
        return Ok(());
    }

    if original_cmd == "giss-list" {
        let acl = AclFile::load()?;
        for repo in acl.repos {
            if repo.name.starts_with(&format!("{}/", user))
                || repo.read.contains(&user.to_string())
                || repo.write.contains(&user.to_string())
            {
                let has_write = repo.name.starts_with(&format!("{}/", user))
                    || repo.write.contains(&user.to_string());
                let access = if has_write { "RW" } else { "R " };
                println!("{} {}", access, repo.name);
            }
        }
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-create ") {
        let repo_name = args.trim();
        let full_name = format!("{}/{}", user, repo_name);
        shared::validate_name(full_name.as_str(), "Repo").map_err(|e| anyhow::anyhow!(e))?;

        let home = std::env::var("HOME")?;
        let path = format!("{}/repos/{}.git", home, full_name);

        if let Some(parent) = std::path::Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        Command::new("git")
            .args(["init", "--bare", &path])
            .status()?;

        let mut acl = AclFile::load()?;
        acl.grant(&full_name, user, true);
        acl.save()?;

        println!("Repository '{}' created", full_name);
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-delete ") {
        let repo_name = args.trim();
        let full_name = if repo_name.contains('/') {
            repo_name.to_string()
        } else {
            format!("{}/{}", user, repo_name)
        };

        let acl = AclFile::load()?;
        let is_owner = full_name.starts_with(&format!("{}/", user));

        if !is_owner && !acl.check(&full_name, user, true) {
            eprintln!(
                "Access denied: you do not have write access to '{}'",
                full_name
            );
            std::process::exit(1);
        }

        let home = std::env::var("HOME")?;
        let path = format!("{}/repos/{}.git", home, full_name);

        if std::path::Path::new(&path).exists() {
            std::fs::remove_dir_all(&path)?;
        }

        let mut acl = acl;
        acl.repos.retain(|r| r.name != full_name);
        acl.save()?;

        println!("Repository '{}' deleted", full_name);
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-grant ") {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() < 2 {
            bail!("Usage: giss-grant <repo> <user> [--write]");
        }

        let repo = parts[0].to_string();
        let target_user = parts[1].to_string();
        let write = parts.contains(&"--write");

        let owner_prefix = format!("{}/", user);
        if !repo.starts_with(&owner_prefix) {
            eprintln!(
                "Access denied: you can only manage access for repositories in your namespace ({})",
                owner_prefix
            );
            std::process::exit(1);
        }

        let mut acl = AclFile::load()?;
        acl.grant(&repo, &target_user, write);
        acl.save()?;

        println!(
            "Granted {} access to '{}' for user '{}'",
            if write { "write" } else { "read" },
            repo,
            target_user
        );
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-revoke ") {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() < 2 {
            bail!("Usage: giss-revoke <repo> <user>");
        }

        let repo = parts[0].to_string();
        let target_user = parts[1].to_string();

        let owner_prefix = format!("{}/", user);
        if !repo.starts_with(&owner_prefix) {
            eprintln!(
                "Access denied: you can only manage access for repositories in your namespace ({})",
                owner_prefix
            );
            std::process::exit(1);
        }

        if target_user == user {
            eprintln!(
                "You cannot revoke your own access. Repository owners always have full access."
            );
            std::process::exit(1);
        }

        let mut acl = AclFile::load()?;
        acl.revoke(&repo, &target_user);
        acl.save()?;

        println!("Revoked access to '{}' for user '{}'", repo, target_user);
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-tui-ls ") {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let mut repo = parts[0].to_string();
        let path = parts.get(1).map(|s| s.to_string()).unwrap_or_default();

        if !repo.contains('/') {
            repo = format!("{}/{}", user, repo);
        }

        if !repo.starts_with(&format!("{}/", user)) && !acl_check(&repo, user, false) {
            eprintln!("Access denied");
            std::process::exit(1);
        }

        let home = std::env::var("HOME")?;
        let full_path = format!("{}/repos/{}.git", home, repo);

        let treeish = if path.is_empty() {
            "HEAD".to_string()
        } else {
            format!("HEAD:{}", path)
        };

        let output = Command::new("git")
            .args(["ls-tree", &treeish])
            .current_dir(&full_path)
            .output()?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.splitn(4, '\t').collect();
            if parts.len() == 2 {
                let meta: Vec<&str> = parts[0].split_whitespace().collect();
                let typ = meta.get(1).unwrap_or(&"blob");
                let name = parts[1];
                println!("{} {}", typ, name);
            }
        }
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-tui-log ") {
        let mut repo = args.trim().to_string();
        if !repo.contains('/') {
            repo = format!("{}/{}", user, repo);
        }

        if !repo.starts_with(&format!("{}/", user)) && !acl_check(&repo, user, false) {
            eprintln!("Access denied");
            std::process::exit(1);
        }

        let home = std::env::var("HOME")?;
        let full_path = format!("{}/repos/{}.git", home, repo);

        let output = Command::new("git")
            .args(["log", "--pretty=format:%H|%an|%ar|%s"])
            .current_dir(&full_path)
            .output()?;

        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-tui-show ") {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let mut repo = parts[0].to_string();
        let file_path = parts.get(1).copied().unwrap_or("");

        if !repo.contains('/') {
            repo = format!("{}/{}", user, repo);
        }

        if !repo.starts_with(&format!("{}/", user)) && !acl_check(&repo, user, false) {
            eprintln!("Access denied");
            std::process::exit(1);
        }

        let home = std::env::var("HOME")?;
        let full_path = format!("{}/repos/{}.git", home, repo);

        let output = Command::new("git")
            .args(["show", &format!("HEAD:{}", file_path)])
            .current_dir(&full_path)
            .output()?;

        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-tui-log ") {
        let mut repo = args.trim().to_string();
        if !repo.contains('/') {
            repo = format!("{}/{}", user, repo);
        }

        if !repo.starts_with(&format!("{}/", user)) && !acl_check(&repo, user, false) {
            eprintln!("Access denied");
            std::process::exit(1);
        }

        let home = std::env::var("HOME")?;
        let full_path = format!("{}/repos/{}.git", home, repo);

        let output = Command::new("git")
            .args(["log", "--pretty=format:%H|%an|%ar|%s"])
            .current_dir(&full_path)
            .output()?;

        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(());
    }

    if let Some(args) = original_cmd.strip_prefix("giss-tui-commit-show ") {
        let parts: Vec<&str> = args.splitn(2, ' ').collect();
        let mut repo = parts[0].to_string();
        let hash = parts.get(1).copied().unwrap_or("");

        if !repo.contains('/') {
            repo = format!("{}/{}", user, repo);
        }

        if !repo.starts_with(&format!("{}/", user)) && !acl_check(&repo, user, false) {
            eprintln!("Access denied");
            std::process::exit(1);
        }

        let home = std::env::var("HOME")?;
        let full_path = format!("{}/repos/{}.git", home, repo);

        let output = Command::new("git")
            .args(["show", hash])
            .current_dir(&full_path)
            .output()?;

        print!("{}", String::from_utf8_lossy(&output.stdout));
        return Ok(());
    }

    let (git_cmd, repo_path) = if let Some(r) = original_cmd.strip_prefix("git-upload-pack '") {
        ("git-upload-pack", r.trim_end_matches('\''))
    } else if let Some(r) = original_cmd.strip_prefix("git-receive-pack '") {
        ("git-receive-pack", r.trim_end_matches('\''))
    } else if let Some(r) = original_cmd.strip_prefix("git-upload-archive '") {
        ("git-upload-archive", r.trim_end_matches('\''))
    } else {
        bail!("Command not allowed");
    };

    let mut repo_name = repo_path.trim_end_matches(".git").to_string();

    if !repo_name.contains('/') {
        repo_name = format!("{}/{}", user, repo_name);
    }

    if repo_name.contains("..") {
        bail!("Invalid repository path");
    }

    let acl = AclFile::load()?;
    let needs_write = git_cmd == "git-receive-pack";

    let is_owner = repo_name.starts_with(&format!("{}/", user));

    if !is_owner && !acl.check(&repo_name, user, needs_write) {
        eprintln!("Access denied for user '{}' to repo '{}'", user, repo_name);
        std::process::exit(1);
    }

    let home = std::env::var("HOME")?;
    let repos_base = std::env::var("GIS_REPOS_PATH").unwrap_or_else(|_| format!("{}/repos", home));
    let full_path = format!("{}/{}.git", repos_base, repo_name);

    let status = Command::new(git_cmd).arg(&full_path).status()?;
    if !status.success() {
        std::process::exit(status.code().unwrap_or(1));
    }
    Ok(())
}

fn acl_check(repo: &str, user: &str, write: bool) -> bool {
    if let Ok(acl) = AclFile::load() {
        return acl.check(repo, user, write);
    }
    false
}
