use anyhow::Result;
use shared::validate_name;
use std::path::PathBuf;

pub fn keys_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME")?;
    let dir = PathBuf::from(home).join(".ssh/keys");
    if !dir.exists() {
        std::fs::create_dir_all(&dir)?;
    }
    Ok(dir)
}

pub fn rebuild_authorized_keys() -> Result<()> {
    let dir = keys_dir()?;
    let home = std::env::var("HOME")?;
    let auth_keys = PathBuf::from(&home).join(".ssh/authorized_keys");

    let gism_path = which::which("gism").unwrap_or_else(|_| PathBuf::from("gism"));

    let mut content = String::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("pub") {
            let user = path.file_stem().and_then(|s| s.to_str()).unwrap_or("");
            if validate_name(user, "User").is_ok() {
                let key = std::fs::read_to_string(&path)?;
                let opts = "no-port-forwarding,no-X11-forwarding,no-agent-forwarding,no-pty";
                content.push_str(&format!(
                    "command=\"{} shell {}\",{} {}\n",
                    gism_path.display(),
                    user,
                    opts,
                    key.trim()
                ));
            }
        }
    }
    std::fs::write(&auth_keys, content)?;
    Ok(())
}

pub fn user_add(name: &str, pubkey: &str) -> Result<()> {
    validate_name(name, "User").map_err(anyhow::Error::msg)?;
    shared::validate_pubkey(pubkey).map_err(anyhow::Error::msg)?;
    let key_path = keys_dir()?.join(format!("{}.pub", name));
    std::fs::write(&key_path, pubkey.trim())?;
    rebuild_authorized_keys()?;
    Ok(())
}

pub fn user_remove(name: &str) -> Result<()> {
    let key_path = keys_dir()?.join(format!("{}.pub", name));
    if key_path.exists() {
        std::fs::remove_file(&key_path)?;
    }
    rebuild_authorized_keys()?;
    Ok(())
}
