use std::{env, fs, path::PathBuf};

use ratatui::{
    DefaultTerminal,
    crossterm::{self, event::KeyCode},
    style::Style,
    widgets::{Block, List, ListDirection, ListState},
};

pub fn get_dirs(current_dir: PathBuf, show_dirs: bool) -> Vec<String> {
    let mut dirs: Vec<_> = fs::read_dir(current_dir)
        .unwrap()
        .filter_map(|dir| {
            let dir = dir.unwrap();
            let filename = dir.file_name().into_string().unwrap();
            if (show_dirs || !dir.path().is_dir()) && filename.chars().next().unwrap() != '.' {
                Some(filename)
            } else {
                None
            }
        })
        .collect();
    dirs.sort();
    dirs
}

pub fn app(terminal: &mut DefaultTerminal) -> std::io::Result<Option<PathBuf>> {
    let mut current_dir = env::current_dir().unwrap();
    let mut dirs = get_dirs(current_dir.clone(), true);

    let mut list = List::new(dirs.clone())
        .block(Block::bordered().title("Files"))
        .style(Style::new().white())
        .highlight_style(Style::new().bold())
        .highlight_symbol("▶ ")
        .direction(ListDirection::TopToBottom);

    let mut state = ListState::default();
    state.select(Some(0));

    loop {
        terminal.draw(|frame| {
            let area = frame.area();
            frame.render_stateful_widget(&list, area, &mut state);
        })?;

        if let Some(event) = crossterm::event::read()?.as_key_press_event() {
            match event.code {
                KeyCode::Esc | KeyCode::Char('q') => break Ok(None),
                KeyCode::Backspace => {
                    current_dir.pop();

                    if current_dir.is_dir() {
                        dirs = get_dirs(current_dir.clone(), true);
                        list = list.items(dirs.clone());
                    } else {
                        break Ok(Some(current_dir));
                    }
                }
                KeyCode::Enter => {
                    if let Some(i) = state.selected() {
                        current_dir.push(dirs[i].clone());

                        if current_dir.is_dir() {
                            dirs = get_dirs(current_dir.clone(), true);
                            list = list.items(dirs.clone());
                        } else {
                            break Ok(Some(current_dir));
                        }
                    }
                }
                KeyCode::Down => {
                    let i = match state.selected() {
                        Some(i) => {
                            if i >= dirs.len() - 1 {
                                0
                            } else {
                                i + 1
                            }
                        }
                        None => 0,
                    };
                    state.select(Some(i));
                }
                KeyCode::Up => {
                    let i = match state.selected() {
                        Some(i) => {
                            if i == 0 {
                                dirs.len() - 1
                            } else {
                                i - 1
                            }
                        }
                        None => 0,
                    };
                    state.select(Some(i));
                }
                _ => (),
            }
        }
    }
}
