use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
pub struct ClientConfig {
    pub host: String,
}

pub fn validate_name(name: &str, what: &str) -> Result<(), String> {
    if name.is_empty() {
        return Err(format!("{} name is empty", what));
    }
    if name.len() > 100 {
        return Err(format!("{} name too long", what));
    }
    if name.starts_with('.') || name.ends_with('.') {
        return Err(format!("{} name cannot start or end with '.'", what));
    }
    if name.starts_with('/') || name.ends_with('/') {
        return Err(format!("{} name cannot start or end with '/'", what));
    }
    if name.contains("..") {
        return Err(format!("{} name cannot contain '..'", what));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' || c == '/')
    {
        return Err(format!("{} name contains invalid characters", what));
    }
    Ok(())
}

pub fn validate_pubkey(key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        return Err("Public key is empty".to_string());
    }
    let parts: Vec<&str> = key.split_whitespace().collect();
    if parts.len() < 2 {
        return Err("Invalid key format".to_string());
    }

    const VALID_TYPES: &[&str] = &[
        "ssh-rsa",
        "ssh-ed25519",
        "ssh-dss",
        "ecdsa-sha2-nistp256",
        "ecdsa-sha2-nistp384",
        "ecdsa-sha2-nistp521",
        "sk-ecdsa-sha2-nistp256@openssh.com",
        "sk-ssh-ed25519@openssh.com",
    ];

    if !VALID_TYPES.contains(&parts[0]) {
        return Err(format!("Unknown ssh key type: {}", parts[0]));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names() {
        assert!(validate_name("my-repo", "Repo").is_ok());
        assert!(validate_name("alice/repo_1", "Repo").is_ok());
        assert!(validate_name("user.name/repo", "Repo").is_ok());
    }

    #[test]
    fn invalid_names() {
        assert!(validate_name("", "Repo").is_err());
        assert!(validate_name("repo/../etc", "Repo").is_err());
        assert!(validate_name("../repo", "Repo").is_err());
        assert!(validate_name("/etc/passwd", "Repo").is_err());
        assert!(validate_name("repo/", "Repo").is_err());
        assert!(validate_name("repo!", "Repo").is_err());
    }

    #[test]
    fn valid_pubkeys() {
        let key = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIabcdefg user@host";
        assert!(validate_pubkey(key).is_ok());
        assert!(validate_pubkey("ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQ").is_ok());
    }

    #[test]
    fn invalid_pubkeys() {
        assert!(validate_pubkey("").is_err());
        assert!(validate_pubkey("ssh-ed25519").is_err());
        assert!(validate_pubkey("unknown-type AAAA").is_err());
    }
}
