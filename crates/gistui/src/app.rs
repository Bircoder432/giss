use crate::git::{Commit, Entry, EntryType, Lines, Panel, View};
use crate::worker::{spawn_worker, Msg, WorkerResult};
use ratatui::text::Line;
use shared::ClientConfig;
use std::collections::HashMap;
use std::process::Command;

pub struct App {
    pub config: ClientConfig,
    pub repos: Vec<String>,
    pub repo_state: ratatui::widgets::ListState,

    pub current_repo: Option<String>,
    pub current_path: String,
    pub entries: Vec<Entry>,
    pub entry_state: ratatui::widgets::ListState,

    pub commits: Vec<Commit>,
    pub commit_state: ratatui::widgets::ListState,

    pub file_content: Lines,
    pub file_cache: HashMap<String, Lines>,
    pub dir_cache: HashMap<String, Vec<Entry>>,
    pub commit_diff_cache: HashMap<String, Lines>,

    pub view: View,
    pub active_panel: Panel,
    pub scroll: u16,

    pub tx: std::sync::mpsc::Sender<Msg>,
    pub rx: std::sync::mpsc::Receiver<WorkerResult>,
}

impl App {
    pub fn new(config: ClientConfig) -> Self {
        let host = config.host.clone();
        let (tx, rx) = spawn_worker(host);

        let mut app = Self {
            config,
            repos: Vec::new(),
            repo_state: ratatui::widgets::ListState::default(),
            current_repo: None,
            current_path: String::new(),
            entries: Vec::new(),
            entry_state: ratatui::widgets::ListState::default(),
            commits: Vec::new(),
            commit_state: ratatui::widgets::ListState::default(),
            file_content: vec![Line::from("Select a file to view")],
            file_cache: HashMap::new(),
            dir_cache: HashMap::new(),
            commit_diff_cache: HashMap::new(),
            view: View::Files,
            active_panel: Panel::Left,
            scroll: 0,
            tx,
            rx,
        };
        app.load_repos();
        app
    }

    pub fn run_ssh(&self, cmd: &str) -> String {
        let output = Command::new("ssh")
            .args([&self.config.host, cmd])
            .output()
            .expect("Failed to run ssh");
        String::from_utf8_lossy(&output.stdout).to_string()
    }

    pub fn load_repos(&mut self) {
        let out = self.run_ssh("giss-list");
        self.repos = out
            .lines()
            .map(|l| l.split_whitespace().nth(1).unwrap_or("").to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !self.repos.is_empty() {
            self.repo_state.select(Some(0));
        }
    }

    pub fn open_repo(&mut self, repo: &str) {
        self.current_repo = Some(repo.to_string());
        self.current_path = String::new();
        self.file_cache.clear();
        self.dir_cache.clear();
        self.commit_diff_cache.clear();
        self.load_files();
        let _ = self.tx.send(Msg::FetchCommits(repo.to_string()));
    }

    pub fn load_files(&mut self) {
        self.entries.clear();
        if let Some(repo) = &self.current_repo {
            if !self.current_path.is_empty() {
                self.entries.push(Entry {
                    name: "..".to_string(),
                    typ: EntryType::UpDir,
                });
            }

            if let Some(cached) = self.dir_cache.get(&self.current_path).cloned() {
                self.entries.extend(cached);
            } else {
                let cmd = format!("giss-tui-ls {} {}", repo, self.current_path);
                let out = self.run_ssh(&cmd);
                let mut fetched = Vec::new();
                for line in out.lines() {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    if parts.len() == 2 {
                        let typ = parts[0];
                        let name = parts[1];
                        if name != "." && name != ".." {
                            let entry_type = if typ == "tree" {
                                EntryType::Dir
                            } else {
                                EntryType::File
                            };
                            let e = Entry {
                                name: name.to_string(),
                                typ: entry_type,
                            };
                            fetched.push(e.clone());
                            self.entries.push(e);
                        }
                    }
                }
                self.dir_cache.insert(self.current_path.clone(), fetched);
            }

            if !self.entries.is_empty() {
                self.entry_state.select(Some(0));
            } else {
                self.entry_state.select(None);
            }
            self.update_preview();
        }
    }

    pub fn get_full_path(&self, entry: &Entry) -> String {
        if self.current_path.is_empty() {
            entry.name.clone()
        } else {
            format!("{}/{}", self.current_path, entry.name)
        }
    }

    pub fn update_preview(&mut self) {
        self.scroll = 0;
        if self.view == View::Files {
            if let Some(idx) = self.entry_state.selected() {
                if let Some(entry) = self.entries.get(idx).cloned() {
                    match entry.typ {
                        EntryType::File => {
                            let full_path = self.get_full_path(&entry);
                            if let Some(cached) = self.file_cache.get(&full_path).cloned() {
                                self.file_content = cached;
                            } else {
                                self.file_content = vec![Line::from("Loading file...")];
                                if let Some(repo) = &self.current_repo {
                                    let _ = self.tx.send(Msg::FetchFile(repo.clone(), full_path));
                                }
                            }
                        }
                        EntryType::Dir => {
                            let full_path = self.get_full_path(&entry);
                            if let Some(cached) = self.dir_cache.get(&full_path).cloned() {
                                let mut lines = Vec::new();
                                for e in cached {
                                    let icon = if e.typ == EntryType::Dir { ">" } else { " " };
                                    lines.push(Line::from(format!("{} {}", icon, e.name)));
                                }
                                if lines.is_empty() {
                                    self.file_content = vec![Line::from("Empty directory")];
                                } else {
                                    self.file_content = lines;
                                }
                            } else {
                                self.file_content =
                                    vec![Line::from("Loading directory preview...")];
                                if let Some(repo) = &self.current_repo {
                                    let _ = self.tx.send(Msg::FetchDir(repo.clone(), full_path));
                                }
                            }
                        }
                        EntryType::UpDir => {
                            self.file_content = vec![Line::from(".. (Go up to parent directory)")];
                        }
                    }
                }
            }
        } else if self.view == View::Commits {
            if let Some(idx) = self.commit_state.selected() {
                if let Some(commit) = self.commits.get(idx).cloned() {
                    let hash = commit.hash.clone();
                    if let Some(cached) = self.commit_diff_cache.get(&hash).cloned() {
                        self.file_content = cached;
                    } else {
                        self.file_content = vec![Line::from("Loading commit diff...")];
                        if let Some(repo) = &self.current_repo {
                            let _ = self.tx.send(Msg::FetchCommitDiff(repo.clone(), hash));
                        }
                    }
                }
            }
        }
    }

    pub fn enter(&mut self) {
        if self.active_panel == Panel::Right {
            return;
        }
        if self.view == View::Files {
            if let Some(idx) = self.entry_state.selected() {
                if let Some(entry) = self.entries.get(idx).cloned() {
                    match entry.typ {
                        EntryType::Dir => {
                            self.current_path = self.get_full_path(&entry);
                            self.load_files();
                        }
                        EntryType::UpDir => {
                            self.go_up();
                        }
                        EntryType::File => {}
                    }
                }
            }
        }
    }

    pub fn go_up(&mut self) {
        if !self.current_path.is_empty() {
            if let Some((parent, _)) = self.current_path.rsplit_once('/') {
                self.current_path = parent.to_string();
            } else {
                self.current_path = String::new();
            }
            self.load_files();
        }
    }

    pub fn next(&mut self) {
        if self.current_repo.is_none() {
            let i = self.repo_state.selected().unwrap_or(0);
            if i + 1 < self.repos.len() {
                self.repo_state.select(Some(i + 1));
            }
            return;
        }

        let state = if self.view == View::Files { &mut self.entry_state } else { &mut self.commit_state };
        let data_len = if self.view == View::Files { self.entries.len() } else { self.commits.len() };

        let i = state.selected().unwrap_or(0);
        if i + 1 < data_len {
            state.select(Some(i + 1));
            self.update_preview();
        }
    }

    pub fn previous(&mut self) {
        if self.current_repo.is_none() {
            let i = self.repo_state.selected().unwrap_or(0);
            if i > 0 {
                self.repo_state.select(Some(i - 1));
            }
            return;
        }

        let state = if self.view == View::Files { &mut self.entry_state } else { &mut self.commit_state };
        let i = state.selected().unwrap_or(0);
        if i > 0 {
            state.select(Some(i - 1));
            self.update_preview();
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(3);
    }
    pub fn scroll_up(&mut self) {
        self.scroll = self.scroll.saturating_sub(3);
    }

    pub fn switch_view(&mut self, view: View) {
        self.view = view.clone();
        self.scroll = 0;
        if self.view == View::Commits
            && !self.commits.is_empty()
            && self.commit_state.selected().is_none()
        {
            self.commit_state.select(Some(0));
        }
        self.update_preview();
    }
}
