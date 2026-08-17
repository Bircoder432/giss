use ratatui::text::Line;

#[derive(Clone, PartialEq)]
pub enum EntryType {
    Dir,
    File,
    UpDir,
}

#[derive(Clone)]
pub struct Entry {
    pub name: String,
    pub typ: EntryType,
}

#[derive(Clone)]
pub struct Commit {
    pub hash: String,
    pub author: String,
    pub date: String,
    pub message: String,
}

#[derive(PartialEq, Clone)]
pub enum View {
    Files,
    Commits,
}

#[derive(PartialEq, Clone)]
pub enum Panel {
    Left,
    Right,
}

pub type Lines = Vec<Line<'static>>;
