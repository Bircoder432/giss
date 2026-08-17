use anyhow::Result;
use clap::{Parser, Subcommand};
use shared::ClientConfig;

#[derive(Parser)]
#[command(name = "giss")]
struct Cli {
    #[command(subcommand)]
    cmd: GissCmd,

    #[arg(short, long, default_value = "~/.config/giss/config.toml")]
    config: String,
}

#[derive(Subcommand)]
enum GissCmd {
    /// List repositories available to you
    List,
    /// Create a new repository under your namespace
    Create { name: String },
    /// Delete a repository (you must have write access)
    Delete { name: String },
    /// Clone a repository
    Clone { name: String },
    /// Add remote to current local git repository
    AddRemote {
        name: String,
        /// Remote name (default: origin)
        #[arg(short, long, default_value = "origin")]
        remote: String,
    },
    /// Grant access to your repository to another user
    Grant {
        repo: String,
        user: String,
        #[arg(long)]
        write: bool,
    },
    /// Revoke access from your repository
    Revoke { repo: String, user: String },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let cfg_path = expand_tilde(&cli.config);
    let cfg_str = std::fs::read_to_string(cfg_path)?;
    let cfg: ClientConfig = toml::from_str(&cfg_str)?;

    match cli.cmd {
        GissCmd::List => {
            let status = std::process::Command::new("ssh")
                .args([&cfg.host, "giss-list"])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to list repositories");
            }
        }
        GissCmd::Create { name } => {
            let cmd_str = format!("giss-create {}", name);
            let status = std::process::Command::new("ssh")
                .args([&cfg.host, &cmd_str])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to create repository");
            }
        }
        GissCmd::Delete { name } => {
            let cmd_str = format!("giss-delete {}", name);
            let status = std::process::Command::new("ssh")
                .args([&cfg.host, &cmd_str])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to delete repository");
            }
        }
        GissCmd::Clone { name } => {
            let url = format!("{}:{}.git", cfg.host, name);
            let status = std::process::Command::new("git")
                .args(["clone", &url])
                .status()?;
            if !status.success() {
                anyhow::bail!("git clone failed");
            }
        }
        GissCmd::AddRemote { name, remote } => {
            let url = format!("{}:{}.git", cfg.host, name);
            let status = std::process::Command::new("git")
                .args(["remote", "add", &remote, &url])
                .status()?;
            if !status.success() {
                anyhow::bail!("git remote add failed");
            }
            println!("Added remote '{}' for {}", remote, name);
        }
        GissCmd::Grant { repo, user, write } => {
            let mut cmd_str = format!("giss-grant {} {}", repo, user);
            if write {
                cmd_str.push_str(" --write");
            }

            let status = std::process::Command::new("ssh")
                .args([&cfg.host, &cmd_str])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to grant access");
            }
        }
        GissCmd::Revoke { repo, user } => {
            let cmd_str = format!("giss-revoke {} {}", repo, user);

            let status = std::process::Command::new("ssh")
                .args([&cfg.host, &cmd_str])
                .status()?;
            if !status.success() {
                anyhow::bail!("Failed to revoke access");
            }
        }
    }
    Ok(())
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), &path[2..]);
        }
    }
    path.to_string()
}
