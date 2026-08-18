use crate::git::{Commit, Entry, EntryType, Lines};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use std::process::Command;
use std::sync::mpsc::{Receiver, Sender, channel};
use std::thread;
use std::time::Duration;
use syntect::easy::HighlightLines;
use syntect::highlighting::ThemeSet;
use syntect::parsing::SyntaxSet;
use syntect::util::LinesWithEndings;

pub enum Msg {
    FetchFile(String, String),
    FetchDir(String, String),
    FetchCommits(String),
    FetchCommitDiff(String, String),
    Quit,
}

pub enum WorkerResult {
    File(String, Lines),
    Dir(String, Vec<Entry>),
    Commits(Vec<Commit>),
    CommitDiff(String, Lines),
}

pub fn spawn_worker(host: String) -> (Sender<Msg>, Receiver<WorkerResult>) {
    let (tx_cmd, rx_cmd) = channel::<Msg>();
    let (tx_res, rx_res) = channel::<WorkerResult>();

    thread::spawn(move || {
        let ps = SyntaxSet::load_defaults_newlines();
        let ts = ThemeSet::load_defaults();

        while let Ok(msg) = rx_cmd.recv() {
            let mut current_msg = msg;
            while let Ok(next_msg) = rx_cmd.try_recv() {
                if matches!(next_msg, Msg::Quit) {
                    return;
                }
                current_msg = next_msg;
            }
            thread::sleep(Duration::from_millis(30));
            while let Ok(next_msg) = rx_cmd.try_recv() {
                if matches!(next_msg, Msg::Quit) {
                    return;
                }
                current_msg = next_msg;
            }

            match current_msg {
                Msg::Quit => return,
                Msg::FetchDir(repo, path) => {
                    let cmd = format!("giss-tui-ls {} {}", repo, path);
                    if let Ok(out) = Command::new("ssh").args([&host, &cmd]).output() {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let mut entries = Vec::new();
                        for line in stdout.lines() {
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
                                    entries.push(Entry {
                                        name: name.to_string(),
                                        typ: entry_type,
                                    });
                                }
                            }
                        }
                        let _ = tx_res.send(WorkerResult::Dir(path, entries));
                    }
                }
                Msg::FetchFile(repo, path) => {
                    let cmd = format!("giss-tui-show {} {}", repo, path);
                    if let Ok(out) = Command::new("ssh").args([&host, &cmd]).output() {
                        let content = String::from_utf8_lossy(&out.stdout).to_string();
                        let lines = highlight_text(&content, &path, &ps, &ts);
                        let _ = tx_res.send(WorkerResult::File(path, lines));
                    }
                }
                Msg::FetchCommits(repo) => {
                    let cmd = format!("giss-tui-log {}", repo);
                    if let Ok(out) = Command::new("ssh").args([&host, &cmd]).output() {
                        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                        let mut commits = Vec::new();
                        for line in stdout.lines() {
                            let parts: Vec<&str> = line.splitn(4, '|').collect();
                            if parts.len() == 4 {
                                commits.push(Commit {
                                    hash: parts[0].to_string(),
                                    author: parts[1].to_string(),
                                    date: parts[2].to_string(),
                                    message: parts[3].to_string(),
                                });
                            }
                        }
                        let _ = tx_res.send(WorkerResult::Commits(commits));
                    }
                }
                Msg::FetchCommitDiff(repo, hash) => {
                    let cmd = format!("giss-tui-commit-show {} {}", repo, hash);
                    if let Ok(out) = Command::new("ssh").args([&host, &cmd]).output() {
                        let content = String::from_utf8_lossy(&out.stdout).to_string();
                        let lines = highlight_text(&content, "file.diff", &ps, &ts);
                        let _ = tx_res.send(WorkerResult::CommitDiff(hash, lines));
                    }
                }
            }
        }
    });

    (tx_cmd, rx_res)
}

fn highlight_text(content: &str, path: &str, ps: &SyntaxSet, ts: &ThemeSet) -> Lines {
    let ext = path.rsplit('.').next().unwrap_or("");
    let syntax = if path.ends_with(".diff") || ext == "diff" {
        ps.find_syntax_by_extension("diff")
            .unwrap_or_else(|| ps.find_syntax_plain_text())
    } else {
        ps.find_syntax_by_extension(ext)
            .unwrap_or_else(|| ps.find_syntax_plain_text())
    };

    let mut h = HighlightLines::new(syntax, &ts.themes["base16-ocean.dark"]);
    let mut lines = Vec::new();

    for line in LinesWithEndings::from(content) {
        let regions = h.highlight_line(line, ps).unwrap();
        let mut spans = Vec::new();
        for (style, text) in regions {
            let color = Color::Rgb(style.foreground.r, style.foreground.g, style.foreground.b);
            spans.push(Span::styled(text.to_string(), Style::default().fg(color)));
        }
        lines.push(Line::from(spans));
    }
    lines
}
