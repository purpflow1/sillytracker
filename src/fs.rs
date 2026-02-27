use std::{fs::read_dir, path::PathBuf};

pub fn get_dirs(current_dir: PathBuf, show_dirs: bool) -> Vec<String> {
    let mut dirs: Vec<_> = read_dir(current_dir)
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
