mod acl;
mod keys;
mod shell;

use anyhow::Result;
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "gism")]
struct Cli {
    #[command(subcommand)]
    cmd: GismCmd,
}

#[derive(Subcommand)]
enum GismCmd {
    /// Initialize the server (run once)
    Init,
    /// Manage users
    User {
        #[command(subcommand)]
        cmd: UserCmd,
    },
    /// Manage repositories
    Repo {
        #[command(subcommand)]
        cmd: RepoCmd,
    },
    /// Manage access rights
    Acl {
        #[command(subcommand)]
        cmd: AclCmd,
    },
    /// Internal: called by SSH forced command
    Shell { user: String },
}

#[derive(Subcommand)]
enum UserCmd {
    Add { name: String, key: String },
    Remove { name: String },
    List,
}
#[derive(Subcommand)]
enum RepoCmd {
    Create { name: String },
    Remove { name: String },
    List,
}
#[derive(Subcommand)]
enum AclCmd {
    Grant {
        repo: String,
        user: String,
        #[arg(long)]
        write: bool,
    },
    Revoke {
        repo: String,
        user: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.cmd {
        GismCmd::Init => {
            let home = std::env::var("HOME")?;
            let repos_dir = format!("{}/repos", home);
            std::fs::create_dir_all(&repos_dir)?;
            acl::AclFile::default().save()?;
            println!("Server initialized. Repos path: {}", repos_dir);
        }
        GismCmd::User { cmd } => match cmd {
            UserCmd::Add { name, key } => {
                keys::user_add(&name, &key)?;
                println!("User '{}' added.", name);
            }
            UserCmd::Remove { name } => {
                keys::user_remove(&name)?;
                println!("User '{}' removed.", name);
            }
            UserCmd::List => {
                let dir = keys::keys_dir()?;
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".pub") {
                            println!("{}", name.trim_end_matches(".pub"));
                        }
                    }
                }
            }
        },
        GismCmd::Repo { cmd } => match cmd {
            RepoCmd::Create { name } => {
                shared::validate_name(&name, "Repo").map_err(anyhow::Error::msg)?;
                let home = std::env::var("HOME")?;
                let path = format!("{}/repos/{}.git", home, name);

                if let Some(parent) = std::path::Path::new(&path).parent() {
                    std::fs::create_dir_all(parent)?;
                }

                std::process::Command::new("git")
                    .args(["init", "--bare", &path])
                    .status()?;
                println!("Repo '{}' created.", name);
            }
            RepoCmd::Remove { name } => {
                let home = std::env::var("HOME")?;
                let path = format!("{}/repos/{}.git", home, name);
                std::fs::remove_dir_all(&path)?;
                println!("Repo '{}' removed.", name);
            }
            RepoCmd::List => {
                let home = std::env::var("HOME")?;
                let dir = format!("{}/repos", home);
                for entry in std::fs::read_dir(dir)? {
                    let entry = entry?;
                    if let Some(name) = entry.file_name().to_str() {
                        if name.ends_with(".git") {
                            println!("{}", name.trim_end_matches(".git"));
                        }
                    }
                }
            }
        },
        GismCmd::Acl { cmd } => match cmd {
            AclCmd::Grant { repo, user, write } => {
                let mut acl = acl::AclFile::load()?;
                acl.grant(&repo, &user, write);
                acl.save()?;
                println!("Granted access to '{}' for user '{}'", repo, user);
            }
            AclCmd::Revoke { repo, user } => {
                let mut acl = acl::AclFile::load()?;
                acl.revoke(&repo, &user);
                acl.save()?;
                println!("Revoked access to '{}' for user '{}'", repo, user);
            }
        },
        GismCmd::Shell { user } => {
            shell::run(&user)?;
        }
    }
    Ok(())
}
