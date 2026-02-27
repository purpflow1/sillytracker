mod files;
mod fs;
mod sound;

use std::{error::Error, path::PathBuf, time::Duration};

use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyEventReceiver, GlobalHotKeyManager,
    hotkey::{Code, HotKey},
};
use ratatui::{
    crossterm::{
        self,
        event::{self, KeyCode, MediaKeyCode},
    },
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, List, ListItem, Paragraph},
};

use crate::{
    fs::get_dirs,
    sound::{Player, PlayingStatus},
};

use ratatui::{Frame, Terminal, layout::Alignment};

struct App {
    // Core state
    current_file: PathBuf,
    current_dir: PathBuf,
    playlist: Vec<String>,
    selected: usize,

    // Audio player
    sound: Player,
    error: bool,

    // Global hotkeys
    _hotkey_manager: GlobalHotKeyManager,
    play_pause_hotkey: HotKey,
    next_hotkey: HotKey,
    prev_hotkey: HotKey,
    event_receiver: &'static GlobalHotKeyEventReceiver,
}

impl App {
    fn new(current_file: PathBuf) -> Result<Self, Box<dyn Error>> {
        let current_filename = current_file
            .file_name()
            .ok_or("Invalid file name")?
            .to_str()
            .ok_or("Non-UTF8 filename")?
            .to_string();

        let mut current_dir = current_file.clone();
        current_dir.pop(); // get parent directory

        let playlist = get_dirs(current_dir.clone(), false);

        // Find the index of the current file in the playlist
        let selected = playlist
            .iter()
            .position(|name| name.as_str() == current_filename.as_str())
            .unwrap_or(0);

        // Setup global hotkeys
        let hotkey_manager = GlobalHotKeyManager::new()?;
        let play_pause = HotKey::new(None, Code::MediaPlayPause);
        let next = HotKey::new(None, Code::MediaTrackNext);
        let prev = HotKey::new(None, Code::MediaTrackPrevious);
        hotkey_manager.register(play_pause)?;
        hotkey_manager.register(next)?;
        hotkey_manager.register(prev)?;
        let event_receiver = GlobalHotKeyEvent::receiver();

        // Initialize player
        let mut sound = Player::new()?;
        let mut error = false;
        if let Err(e) = sound.play(current_file.as_path()) {
            sound.title = format!("Error: {}", e);
            error = true;
        }

        Ok(Self {
            current_file,
            current_dir,
            playlist,
            selected,
            sound,
            error,
            _hotkey_manager: hotkey_manager,
            play_pause_hotkey: play_pause,
            next_hotkey: next,
            prev_hotkey: prev,
            event_receiver,
        })
    }

    /// Main loop: handles events, updates state, and draws the UI.
    fn run(
        &mut self,
        terminal: &mut Terminal<impl ratatui::backend::Backend>,
    ) -> Result<(), Box<dyn Error>> {
        loop {
            // Draw the current UI state
            terminal.draw(|f| self.draw(f)).unwrap();

            // Auto‑advance to next track when current finishes
            self.auto_advance()?;

            // Handle global hotkey events (media keys)
            self.handle_global_hotkey()?;

            // Handle keyboard events
            if self.handle_keyboard_event()? {
                break Ok(());
            }

            // Let the player check device status (e.g., speaker changes)
            self.sound.check_device()?;
        }
    }

    /// Draws the entire interface.
    fn draw(&self, f: &mut Frame) {
        let size = f.area();

        // Layout: main area (playlist + now playing), controls, help
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(3),
            ])
            .split(size);

        // Split main area into playlist (70%) and now playing (30%)
        let main_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(chunks[0]);

        self.draw_playlist(f, main_chunks[0]);
        self.draw_now_playing(f, main_chunks[1]);
        self.draw_controls(f, chunks[1]);
        self.draw_help(f, chunks[2]);

        // Warn if terminal is too small
        if size.width < 60 || size.height < 20 {
            self.draw_size_warning(f, size);
        }
    }

    fn draw_playlist(&self, f: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .playlist
            .iter()
            .enumerate()
            .map(|(i, name)| {
                let prefix = if i == self.selected {
                    Span::styled("▶ ", Style::default().fg(Color::Green))
                } else {
                    Span::raw("  ")
                };
                ListItem::new(Line::from(vec![prefix, Span::raw(name.clone())]))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Playlist"))
            .highlight_style(Style::default().bg(Color::Blue));

        f.render_widget(list, area);
    }

    fn draw_now_playing(&self, f: &mut Frame, area: Rect) {
        let title = if self.sound.title.is_empty() {
            self.current_file
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        } else {
            self.sound.title.clone()
        };

        let now_playing = Paragraph::new(vec![
            Line::from(Span::raw(title)),
            Line::from(Span::raw(self.sound.album.as_str())),
            Line::from(vec![Span::styled(
                self.sound.artist.as_str(),
                Style::default().add_modifier(Modifier::BOLD),
            )]),
        ])
        .block(Block::default().borders(Borders::ALL).title("Now Playing"))
        .style(Style::default().fg(Color::White));

        f.render_widget(now_playing, area);
    }

    fn draw_controls(&self, f: &mut Frame, area: Rect) {
        let controls_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage(50), // progress + time
                Constraint::Percentage(25), // play/pause status
                Constraint::Percentage(25), // volume
            ])
            .split(area);

        self.draw_progress(f, controls_chunks[0]);
        self.draw_playback_status(f, controls_chunks[1]);
        self.draw_volume(f, controls_chunks[2]);
    }

    fn draw_progress(&self, f: &mut Frame, area: Rect) {
        let position_secs = self.sound.get_position().as_secs();
        let total_secs = self.sound.duration.as_secs().max(1);
        let ratio = (position_secs as f64 / total_secs as f64).min(1.0);

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(Color::Cyan).bg(Color::Black))
            .ratio(ratio)
            .label(format!(
                "{} / {}",
                Self::format_duration(position_secs),
                Self::format_duration(total_secs)
            ));
        f.render_widget(gauge, area);
    }

    fn draw_playback_status(&self, f: &mut Frame, area: Rect) {
        let status_text = match self.sound.status {
            PlayingStatus::Played => "Playing",
            PlayingStatus::Paused => "Paused",
            PlayingStatus::Stopped => "Stopped",
        };
        let paragraph = Paragraph::new(Line::from(vec![Span::raw(status_text)]))
            .block(Block::default().borders(Borders::ALL).title("Controls"))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }

    fn draw_volume(&self, f: &mut Frame, area: Rect) {
        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Volume"))
            .gauge_style(
                Style::default()
                    .fg(Color::Yellow)
                    .bg(Color::Black)
                    .add_modifier(Modifier::BOLD),
            )
            .ratio(self.sound.volume as f64);
        f.render_widget(gauge, area);
    }

    fn draw_help(&self, f: &mut Frame, area: Rect) {
        let help_text = Line::from(vec![
            Span::raw(" Space [Pause/Play]    "),
            Span::raw("Down [Volume Down]    "),
            Span::raw("Up [Volume Up]    "),
            Span::raw("h [Seek back]    "),
            Span::raw("j [Next]    "),
            Span::raw("k [Previous]    "),
            Span::raw("l [Seek forward]    "),
        ]);
        let paragraph = Paragraph::new(help_text)
            .block(Block::default().borders(Borders::ALL).title("Help"))
            .alignment(Alignment::Center);
        f.render_widget(paragraph, area);
    }

    fn draw_size_warning(&self, f: &mut Frame, size: Rect) {
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

    /// Format seconds as MM:SS.
    fn format_duration(secs: u64) -> String {
        format!("{}:{:02}", secs / 60, secs % 60)
    }

    /// If the current track has finished and there's no error, move to the next track.
    fn auto_advance(&mut self) -> Result<(), Box<dyn Error>> {
        if self.sound.get_position().as_secs_f64() + 0.1 >= self.sound.duration.as_secs_f64()
            && !self.error
        {
            self.selected += 1;
            if self.selected >= self.playlist.len() {
                self.selected = 0;
            }
            self.current_file = self.current_dir.join(&self.playlist[self.selected]);
            let _ = self.sound.play(self.current_file.as_path());
        }
        Ok(())
    }

    /// Handle global hotkey events (media keys).
    fn handle_global_hotkey(&mut self) -> Result<(), Box<dyn Error>> {
        if let Ok(event) = self.event_receiver.try_recv() {
            if event.id() == self.play_pause_hotkey.id() {
                self.toggle_playback();
            } else if event.id() == self.next_hotkey.id() {
                self.play_next()?;
            } else if event.id() == self.prev_hotkey.id() {
                self.play_previous()?;
            }
        }
        Ok(())
    }

    /// Handle keyboard input. Returns `true` if the application should exit.
    fn handle_keyboard_event(&mut self) -> Result<bool, Box<dyn Error>> {
        if event::poll(Duration::from_millis(500))?
            && let Some(event) = crossterm::event::read()?.as_key_press_event()
        {
            match event.code {
                KeyCode::Esc | KeyCode::Char('q') => return Ok(true),
                KeyCode::Up => self.sound.set_volume(self.sound.volume + 0.1),
                KeyCode::Down => self.sound.set_volume(self.sound.volume - 0.1),
                KeyCode::Char(' ') | KeyCode::Media(MediaKeyCode::PlayPause) => {
                    self.toggle_playback()
                }
                KeyCode::Char('j') | KeyCode::Media(MediaKeyCode::TrackNext) => self.play_next()?,
                KeyCode::Char('k') | KeyCode::Media(MediaKeyCode::TrackPrevious) => {
                    self.play_previous()?
                }
                KeyCode::Char('h') => self.sound.move_position(-10.)?,
                KeyCode::Char('l') => self.sound.move_position(10.)?,
                _ => (),
            }
        }
        Ok(false)
    }

    fn toggle_playback(&mut self) {
        match self.sound.status {
            PlayingStatus::Played => self.sound.pause(),
            _ => self.sound.resume(),
        }
    }

    fn play_next(&mut self) -> Result<(), Box<dyn Error>> {
        self.selected += 1;
        if self.selected >= self.playlist.len() {
            self.selected = 0;
        }
        self.load_current_track()
    }

    fn play_previous(&mut self) -> Result<(), Box<dyn Error>> {
        if self.selected == 0 {
            self.selected = self.playlist.len() - 1;
        } else {
            self.selected -= 1;
        }
        self.load_current_track()
    }

    fn load_current_track(&mut self) -> Result<(), Box<dyn Error>> {
        self.current_file = self.current_dir.join(&self.playlist[self.selected]);
        match self.sound.play(self.current_file.as_path()) {
            Ok(_) => self.error = false,
            Err(e) => {
                self.error = true;
                self.sound.title = format!("Error: {}", e);
            }
        }
        Ok(())
    }
}

fn app(current_file: PathBuf) -> Result<(), Box<dyn Error>> {
    let mut app = App::new(current_file)?;
    ratatui::run(|terminal| app.run(terminal))
}

fn main() -> Result<(), Box<dyn Error>> {
    while let Some(path) = ratatui::run(files::app)? {
        app(path)?;
    }

    Ok(())
}
