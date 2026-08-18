mod app;
mod git;
mod ui;
mod worker;

use anyhow::Result;
use app::App;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use shared::ClientConfig;
use std::{io::stdout, time::Duration};

use crate::git::{Panel, View};
use crate::worker::{Msg, WorkerResult};

fn main() -> Result<()> {
    let home = std::env::var("HOME")?;
    let cfg_path = format!("{}/.config/giss/config.toml", home);
    let cfg_str = std::fs::read_to_string(cfg_path)?;
    let config: ClientConfig = toml::from_str(&cfg_str)?;

    let mut app = App::new(config);

    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    loop {
        while let Ok(res) = app.rx.try_recv() {
            match res {
                WorkerResult::File(path, lines) => {
                    app.file_cache.insert(path.clone(), lines.clone());
                    if app.view == View::Files
                        && let Some(idx) = app.entry_state.selected()
                        && let Some(entry) = app.entries.get(idx)
                        && entry.typ == EntryType::File
                        && app.get_full_path(entry) == path
                    {
                        app.file_content = lines;
                    }
                }
                WorkerResult::Dir(path, entries) => {
                    app.dir_cache.insert(path.clone(), entries.clone());
                    if app.view == View::Files
                        && let Some(idx) = app.entry_state.selected()
                        && let Some(entry) = app.entries.get(idx)
                        && entry.typ == EntryType::Dir
                        && app.get_full_path(entry) == path
                    {
                        let mut lines = Vec::new();
                        for e in &entries {
                            let icon = if e.typ == EntryType::Dir { ">" } else { " " };
                            lines.push(ratatui::text::Line::from(format!("{} {}", icon, e.name)));
                        }
                        if lines.is_empty() {
                            app.file_content = vec![ratatui::text::Line::from("Empty directory")];
                        } else {
                            app.file_content = lines;
                        }
                    }
                }
                WorkerResult::Commits(commits) => {
                    app.commits = commits.clone();
                    if app.view == View::Commits && !app.commits.is_empty() {
                        app.commit_state.select(Some(0));
                        app.update_preview();
                    }
                }
                WorkerResult::CommitDiff(hash, lines) => {
                    app.commit_diff_cache.insert(hash.clone(), lines.clone());
                    if app.view == View::Commits
                        && let Some(idx) = app.commit_state.selected()
                        && let Some(c) = app.commits.get(idx)
                        && c.hash == hash
                    {
                        app.file_content = lines;
                    }
                }
            }
        }

        terminal.draw(|f| ui::ui(f, &mut app))?;

        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            match key.code {
                KeyCode::Char('q') => {
                    let _ = app.tx.send(Msg::Quit);
                    break;
                }
                KeyCode::Tab => {
                    app.active_panel = match app.active_panel {
                        Panel::Left => Panel::Right,
                        Panel::Right => Panel::Left,
                    };
                }
                KeyCode::Char('f') => app.switch_view(View::Files),
                KeyCode::Char('c') => app.switch_view(View::Commits),
                KeyCode::Right | KeyCode::Char('l') => {
                    if app.active_panel == Panel::Left {
                        if app.current_repo.is_none() {
                            if let Some(idx) = app.repo_state.selected()
                                && let Some(repo) = app.repos.get(idx).cloned()
                            {
                                app.open_repo(&repo);
                            }
                        } else {
                            app.active_panel = Panel::Right;
                        }
                    }
                }
                KeyCode::Left | KeyCode::Backspace | KeyCode::Char('h') => {
                    if app.active_panel == Panel::Right {
                        app.active_panel = Panel::Left;
                    } else if app.active_panel == Panel::Left && app.current_repo.is_some() {
                        if app.current_path.is_empty() {
                            app.current_repo = None;
                            app.file_content.clear();
                            app.entries.clear();
                            app.commits.clear();
                            app.entry_state.select(None);
                        } else {
                            app.go_up();
                        }
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if app.active_panel == Panel::Left {
                        app.next();
                    } else {
                        app.scroll_down();
                    }
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if app.active_panel == Panel::Left {
                        app.previous();
                    } else {
                        app.scroll_up();
                    }
                }
                KeyCode::Enter => {
                    if app.active_panel == Panel::Left {
                        if app.current_repo.is_none() {
                            if let Some(idx) = app.repo_state.selected()
                                && let Some(repo) = app.repos.get(idx).cloned()
                            {
                                app.open_repo(&repo);
                            }
                        } else {
                            app.enter();
                        }
                    }
                }
                KeyCode::Esc => {
                    app.current_repo = None;
                    app.file_content.clear();
                }
                _ => {}
            }
        }
    }

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

use crate::git::EntryType;
