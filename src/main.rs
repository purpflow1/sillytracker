use std::{error::Error, path::PathBuf, sync::RwLock};

mod sound;
mod ui;

static CURRENT_FILE: RwLock<PathBuf> = RwLock::new(PathBuf::new());

fn main() -> Result<(), Box<dyn Error>> {
    while let Some(path) = ratatui::run(ui::files::app)? {
        *CURRENT_FILE.write().unwrap() = path;

        ratatui::run(ui::player::app)?;
    }

    Ok(())
}
