# GISS (Git over Security Shell)

A lightweight, self-hosted Git repository manager that operates entirely over SSH. It provides per-user access control, repository namespacing, and a terminal UI for browsing code without cloning. No web server, no database, and no daemons required.

## Architecture

The project is split into three separate binaries:

*   `gism` (Server Manager): Installed on the remote VPS. It manages bare repositories, SSH `authorized_keys`, and access control lists (ACL). It acts as the restricted shell for all Git operations.
*   `giss` (User CLI): Installed on the developer's machine. It provides standard Git commands (clone, create, push) and access management by communicating with `gism` over SSH.
*   `gistui` (TUI Client): A terminal user interface for browsing remote repositories, viewing file contents with syntax highlighting, and reading commit diffs without cloning.

## Installation

This project uses a Cargo workspace. To build all binaries, run:

```bash
cargo build --release
```

The compiled binaries will be located in `target/release/`.
### via script

```bash
curl -sL https://raw.githubusercontent.com/bircoder432/giss/master/install.sh | bash
```

## Server Setup (Admin)

1. Create a dedicated system user for Git operations on your VPS:
   ```bash
   sudo useradd -m -s /bin/bash git
   ```

2. Copy the `gism` binary to the server and place it in a directory available in the `git` user's `PATH` (e.g., `/usr/local/bin/gism`).

3. Switch to the `git` user and initialize the server:
   ```bash
   sudo -u git -i
   gism init
   ```
   This creates the `~/repos` directory and the ACL configuration file.

4. Add users and their public SSH keys:
   ```bash
   gism user add alice "ssh-ed25519 AAAAC3NzaC1lZDI1... alice@laptop"
   ```
   `gism` will automatically configure `~/.ssh/authorized_keys` to restrict this key to `gism shell alice`, preventing general shell access.

5. (Optional) Create a shared repository and grant access:
   ```bash
   gism repo create shared-project
   gism acl grant shared-project alice --write
   ```

## Client Setup (User)

1. Copy the `giss` and `gistui` binaries to your local machine.

2. Create the configuration file at `~/.config/giss/config.toml`:
   ```toml
   host = "git@your_server_ip"
   ```

## Usage

### giss (CLI)

Standard commands for repository management:

```bash
# List repositories available to you
giss list

# Create a new repository (automatically prefixed with your username, e.g., alice/myrepo)
giss create myrepo

# Clone a repository
giss clone alice/myrepo

# Add a remote to your current local directory
giss add-remote alice/myrepo

# Add a remote with a custom name
giss add-remote alice/myrepo -r upstream

# Grant access to another user
giss grant alice/myrepo bob --write

# Revoke access
giss revoke alice/myrepo bob
```

### gistui (TUI)

Launch the terminal interface:

```bash
gistui
```

**Keybindings:**

*   `j` / `k` or `Up` / `Down`: Move selection.
*   `Enter` or `l` / `Right`: Open repository or directory.
*   `h` / `Left` or `Backspace`: Go back to parent directory or exit repository.
*   `Tab`: Switch focus between the file tree and the preview panel.
*   `f`: Switch to Files view.
*   `c`: Switch to Commits view.
*   `q`: Quit.

When the right panel is active, `j` / `k` will scroll the file content or commit diff.

## Security Model

When a user connects via SSH, the server executes `gism shell <username>` using the `command=` prefix in `authorized_keys`. 

The `gism` shell intercepts the `SSH_ORIGINAL_COMMAND` environment variable. It only permits standard Git commands (`git-upload-pack`, `git-receive-pack`) and specific `giss-*` commands. 

Access control is enforced before any Git command is executed:
*   Users automatically own repositories created within their namespace (e.g., `alice/repo`). They cannot revoke their own access.
*   Users can only grant or revoke access to repositories within their own namespace.
*   Raw shell access to the server is denied.
