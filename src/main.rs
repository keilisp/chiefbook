extern crate termion;
mod event;
use crate::event::{Event, Events};

use pipers::Pipe;
use std::error::Error;
use std::io::stdout;
use std::path::Path;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use termion::{event::Key, input::MouseTerminal, raw::IntoRawMode, screen::AlternateScreen};

use std::ffi::OsStr;
use std::ffi::OsString;

use tui::{
    backend::TermionBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Terminal,
};

extern crate regex;

use regex::Regex;

fn main() -> Result<(), Box<dyn Error>> {
    // #[derive(Debug)]
    struct Document {
        path: PathBuf,
        name: String,
        ext: String,
        author: String,
        year: String,
        // total_pages: u32,
        total_pages: String,
        current_page: u32,
    }

    impl<'a> Document {
        fn new(
            path: PathBuf,
            name: String,
            ext: String,
            author: String,
            year: String,
            // total_pages: u32,
            total_pages: String,
            current_page: u32,
        ) -> Self {
            Document {
                path,
                name,
                ext,
                author,
                year,
                total_pages,
                current_page,
            }
        }
    }

    // #[derive(Debug)]
    struct Documents {
        items: Vec<Document>,
        state: ListState,
    }

    impl<'a> Documents {
        fn new(folder: String) -> Self {
            let path = Path::new(&folder);
            let mut items: Vec<Document> = Vec::new();
            for entry in path
                .read_dir()
                .expect("Something went wrong during  reading the directory")
            {
                if let Ok(entry) = entry {
                    let file_path = entry.path();
                    let file_name = entry.file_name();
                    let file_ext = file_path.extension().unwrap().to_str().unwrap().to_string();
                    let file_name_noext =
                        file_path.file_stem().unwrap().to_str().unwrap().to_string();

                    let exiftool_cmd_author =
                        format!("exiftool -Author {}", file_path.to_str().unwrap());

                    let author_out = Pipe::new(&exiftool_cmd_author)
                        .then("cut -d : -f2")
                        .finally()
                        .expect("Commands did not pipe")
                        .wait_with_output()
                        .expect("Failed to wait on child");

                    let author_str = String::from_utf8(author_out.stdout).unwrap();

                    let mut author = author_str.trim();
                    if author.is_empty() {
                        author = "Unknown";
                    }

                    let exiftool_cmd_pages =
                        format!("exiftool -PageCount {}", file_path.to_str().unwrap());

                    let pages_out = Pipe::new(&exiftool_cmd_pages)
                        .then("cut -d : -f2")
                        .finally()
                        .expect("Commands did not pipe")
                        .wait_with_output()
                        .expect("Failed to wait on child");

                    let pages_count_str = String::from_utf8(pages_out.stdout).unwrap();

                    let mut pages_count = pages_count_str.trim();
                    if pages_count.is_empty() {
                        pages_count = "Unknown";
                    }

                    let document = Document::new(
                        file_path.to_path_buf(),
                        file_name_noext,
                        file_ext,
                        author.to_string(),
                        "1984".to_string(),
                        pages_count.to_string(),
                        32,
                    );

                    items.push(document);
                }
            }

            Documents {
                items,
                state: ListState::default(),
            }
        }

        pub fn next(&mut self) {
            let i = match self.state.selected() {
                Some(i) => {
                    if i >= self.items.len() - 1 {
                        0
                    } else {
                        i + 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }
        pub fn prev(&mut self) {
            let i = match self.state.selected() {
                Some(i) => {
                    if i == 0 {
                        self.items.len()
                    } else {
                        i - 1
                    }
                }
                None => 0,
            };
            self.state.select(Some(i));
        }

        pub fn open(&self) {
            let current_doc = &self.items[self.state.selected().unwrap()];
            if current_doc.ext == "pdf" {
                Command::new("zathura")
                    .arg(&current_doc.path)
                    .stderr(Stdio::null())
                    .status()
                    .expect("Can't open this file!");
            } else {
                Command::new("xdg-open")
                    .arg(&current_doc.path)
                    .stderr(Stdio::null())
                    .status()
                    .expect("shieeet");
            }
        }
        pub fn unselect(&mut self) {
            self.state.select(None);
        }
    }

    let stdout = stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;
    let events = Events::new();

    terminal.clear().unwrap();

    let mut documents = Documents::new("/home/mediocre/dox/2read".to_string());
    // let mut documents = Documents::new("/home/mediocre/dox/test".to_string());
    documents.state.select(Some(0));

    loop {
        terminal.draw(|f| {
            // let chunks = Layout::default()
            //     .direction(Direction::Horizontal)
            //     .margin(2)
            //     .constraints([Constraint::Percentage(60), Constraint::Percentage(40)].as_ref())
            //     .split(f.size());

            let documents_items: Vec<ListItem> = documents
                .items
                .iter()
                .map(|i| ListItem::new(i.name.as_ref()))
                .collect();

            let documents_list = List::new(documents_items)
                .block(Block::default().borders(Borders::ALL).title("Documents"))
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .bg(Color::Yellow)
                        .add_modifier(Modifier::ITALIC),
                )
                .highlight_symbol(">> ");

            f.render_stateful_widget(documents_list, f.size(), &mut documents.state);
        })?;

        match events.next().unwrap() {
            Event::Input(input) => match input {
                Key::Char('q') => break,
                Key::Char('j') | Key::Up => documents.next(),
                Key::Char('k') | Key::Down => documents.prev(),
                Key::Char('o') | Key::Char('l') => documents.open(),
                _ => {}
            },
            _ => {}
        }
    }

    Ok(())
}

fn get_files_from_dir(folder: String) -> Vec<String> {
    let path = Path::new(&folder);
    let mut files_vec: Vec<String> = Vec::new();
    for entry in path
        .read_dir()
        .expect("Something went wrong durin reading the directory")
    {
        if let Ok(entry) = entry {
            let file_path = entry.path();
            let _file_name = entry.file_name();
            let _file_ext = file_path.extension().unwrap();
            let file_name_noext = file_path.file_stem().unwrap().to_str().unwrap().to_string();

            files_vec.push(file_name_noext);
        }
    }

    files_vec
}
