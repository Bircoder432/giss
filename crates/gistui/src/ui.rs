use crate::app::App;
use crate::git::{EntryType, Panel, View};
use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::Line;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};
use ratatui::Frame;

pub fn ui(f: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(30), Constraint::Percentage(70)].as_ref())
        .split(f.size());

    let left_style = if app.active_panel == Panel::Left {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let right_style = if app.active_panel == Panel::Right {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    if app.current_repo.is_none() {
        let items: Vec<ListItem> = app
            .repos
            .iter()
            .map(|r| ListItem::new(Line::from(r.clone())))
            .collect();
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Repositories (Enter/Open)")
                    .border_style(left_style),
            )
            .highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            );
        f.render_stateful_widget(list, chunks[0], &mut app.repo_state);

        let para = Paragraph::new("Select a repository on the left to view its files here.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Info")
                    .border_style(right_style),
            )
            .wrap(Wrap { trim: true });
        f.render_widget(para, chunks[1]);
    } else {
        let repo_name = app.current_repo.as_ref().unwrap();
        let view_str = if app.view == View::Files {
            "Files"
        } else {
            "Commits"
        };
        let title = format!("[{}] {} ({})", view_str, repo_name, app.current_path);

        let left_block = Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(left_style);

        if app.view == View::Files {
            let items: Vec<ListItem> = app
                .entries
                .iter()
                .map(|e| {
                    let icon = match e.typ {
                        EntryType::Dir => ">",
                        EntryType::File => " ",
                        EntryType::UpDir => "..",
                    };
                    ListItem::new(Line::from(format!("{} {}", icon, e.name)))
                })
                .collect();

            let list = List::new(items).block(left_block).highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            );
            f.render_stateful_widget(list, chunks[0], &mut app.entry_state);
        } else {
            let items: Vec<ListItem> = app
                .commits
                .iter()
                .map(|c| {
                    let short_hash = &c.hash[..7];
                    ListItem::new(Line::from(format!(
                        "{} [{}] {} - {}",
                        short_hash, c.date, c.author, c.message
                    )))
                })
                .collect();

            let list = List::new(items).block(left_block).highlight_style(
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            );
            f.render_stateful_widget(list, chunks[0], &mut app.commit_state);
        }

        let right_title = if app.view == View::Files {
            "Preview"
        } else {
            "Commit Diff"
        };
        let para = Paragraph::new(app.file_content.clone())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(right_title)
                    .border_style(right_style),
            )
            .scroll((app.scroll, 0))
            .wrap(Wrap { trim: false });
        f.render_widget(para, chunks[1]);
    }
}
