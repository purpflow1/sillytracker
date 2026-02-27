use std::time::Duration;

use ratatui::{
    DefaultTerminal,
    crossterm::{
        self,
        event::{self, KeyCode},
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

use crate::{
    CURRENT_FILE,
    sound::{Player, PlayingStatus},
    ui::files::get_dirs,
};

pub fn app(terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    let mut current_file = CURRENT_FILE.read().unwrap().clone();
    let current_filename = current_file
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_string();
    let mut current_dir = current_file.clone();
    current_dir.pop();
    let playlist = get_dirs(current_dir.clone());

    let mut selected = 0;

    for dir in &playlist {
        if dir.as_str() != current_filename.as_str() {
            selected += 1;
        } else {
            break;
        }
    }

    let mut sound = Player::new().unwrap();
    let mut error = false;
    if let Err(e) = sound.play(current_file.as_path()) {
        sound.title = format!("Error: {}", e);
        error = true;
    }

    loop {
        terminal.draw(|f| {
            let size = f.area();

            let position_secs = sound.get_position().as_secs();
            let total_secs = sound.duration.as_secs().max(1);
            let ratio = (position_secs as f64 / total_secs as f64).min(1.);

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(10),   // main area
                        Constraint::Length(3), // player controls
                        Constraint::Length(3), // help
                    ]
                    .as_ref(),
                )
                .split(size);

            let main_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(70), Constraint::Percentage(30)].as_ref())
                .split(chunks[0]);

            let items: Vec<ListItem> = playlist
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let mut spans = Vec::new();
                    if i == selected {
                        spans.push(Span::styled("▶ ", Style::default().fg(Color::Green)));
                    } else {
                        spans.push(Span::raw("  "));
                    }
                    spans.push(Span::raw(t.clone()));
                    ListItem::new(Line::from(spans))
                })
                .collect();

            let playlist = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Playlist"))
                .highlight_style(Style::default().bg(Color::Blue));

            f.render_widget(playlist, main_chunks[0]);

            let title = if sound.title.len() == 0 {
                current_filename.clone()
            } else {
                sound.title.clone()
            };

            let now_playing = Paragraph::new(vec![
                Line::from(Span::raw(title)),
                Line::from(Span::raw(sound.album.as_str())),
                Line::from(vec![Span::styled(
                    sound.artist.as_str(),
                    Style::default().add_modifier(Modifier::BOLD),
                )]),
            ])
            .block(Block::default().borders(Borders::ALL).title("Now Playing"))
            .style(Style::default().fg(Color::White));

            f.render_widget(now_playing, main_chunks[1]);

            let controls = Layout::default()
                .direction(Direction::Horizontal)
                .constraints(
                    [
                        Constraint::Percentage(50), // progress and time
                        Constraint::Percentage(25), // play buttons
                        Constraint::Percentage(25), // volume
                    ]
                    .as_ref(),
                )
                .split(chunks[1]);

            macro_rules! parse_secs {
                ($secs:expr) => {
                    format!("{}:{:02}", ($secs) / 60, ($secs) % 60)
                };
            }

            // Progress bar + time
            let progress_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Progress"))
                .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
                .ratio(ratio)
                .label(format!(
                    "{} / {}",
                    parse_secs!(position_secs),
                    parse_secs!(total_secs)
                ));

            f.render_widget(progress_gauge, controls[0]);

            // Play/Pause/Skip
            let status_text = match sound.status {
                PlayingStatus::Played => "Playing",
                PlayingStatus::Paused => "Paused",
                PlayingStatus::Stopped => "Stopped",
            };
            let buttons = Paragraph::new(Line::from(vec![Span::raw(status_text)]))
                .block(Block::default().borders(Borders::ALL).title("Controls"))
                .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(buttons, controls[1]);

            // Volume gauge
            let vol_ratio = sound.volume as f64;
            let vol_gauge = Gauge::default()
                .block(Block::default().borders(Borders::ALL).title("Volume"))
                .gauge_style(
                    Style::default()
                        .fg(Color::Yellow)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .ratio(vol_ratio);

            f.render_widget(vol_gauge, controls[2]);

            // Draw borders for whole app if small area
            if size.width < 60 || size.height < 20 {
                let warn = Paragraph::new("Terminal too small")
                    .block(Block::default().borders(Borders::ALL).title("Warning"));
                let area = Rect {
                    x: size.x + 1,
                    y: size.y + 1,
                    width: size.width.saturating_sub(2),
                    height: size.height.saturating_sub(2),
                };
                f.render_widget(warn, area);
            }

            let help = Paragraph::new(Line::from(vec![
                Span::raw(" Space [Pause/Play]    "),
                Span::raw("Down [Volume Down]    "),
                Span::raw("Up [Volume Up]    "),
                Span::raw("h [Seek back]    "),
                Span::raw("j [Next]    "),
                Span::raw("k [Previous]    "),
                Span::raw("l [Seek right]    "),
            ]))
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .alignment(ratatui::layout::Alignment::Center);

            f.render_widget(help, chunks[2]);
        })?;

        if sound.get_position().as_secs_f64() + 0.1 >= sound.duration.as_secs_f64() && !error {
            selected += 1;
            if playlist.len() == selected {
                selected = 0;
            }
            current_file = current_dir.join(playlist[selected].clone());
            if let Err(e) = sound.play(current_file.as_path()) {
                error = true;
                sound.title = format!("Error: {}", e);
            } else {
                error = false;
            }
        }

        if event::poll(Duration::from_millis(16))?
            && let Some(event) = crossterm::event::read()?.as_key_press_event()
        {
            match event.code {
                KeyCode::Esc | KeyCode::Char('q') => break Ok(()),
                KeyCode::Up => sound.set_volume(sound.volume + 0.1),
                KeyCode::Down => sound.set_volume(sound.volume - 0.1),
                KeyCode::Char(' ') => match sound.status {
                    PlayingStatus::Played => sound.pause(),
                    _ => sound.resume(),
                },
                KeyCode::Char('j') => {
                    selected += 1;
                    if playlist.len() == selected {
                        selected = 0;
                    }
                    current_file = current_dir.join(playlist[selected].clone());
                    if let Err(e) = sound.play(current_file.as_path()) {
                        error = true;
                        sound.title = format!("Error: {}", e);
                    } else {
                        error = false;
                    }
                }
                KeyCode::Char('k') => {
                    if selected == 0 {
                        selected = playlist.len() - 1;
                    } else {
                        selected -= 1;
                    }

                    current_file = current_dir.join(playlist[selected].clone());
                    if let Err(e) = sound.play(current_file.as_path()) {
                        error = true;
                        sound.title = format!("Error: {}", e);
                    } else {
                        error = false;
                    }
                }
                KeyCode::Char('h') => {
                    sound.move_position(-10.).unwrap();
                }
                KeyCode::Char('l') => {
                    sound.move_position(10.).unwrap();
                }
                _ => (),
            }
        }

        sound.check_device().unwrap();
    }
}
