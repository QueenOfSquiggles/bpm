use std::{
    collections::VecDeque,
    fs,
    path::{Path, PathBuf},
    time::SystemTime,
};

use crate::{config::Config, raw::ProcessingRaw};
use bevy::prelude::*;
use walkdir::WalkDir;

/// The core component that links an entity to a specific file in the staging directory
#[derive(Component, Debug)]
pub struct FileQueuedForProcessing {
    pub source: PathBuf,
    pub dest: PathBuf,
}

/// The core trait for processing information.
/// Less for dealing with processing types in generic form but for ensuring each processing type meets common constraints
pub trait ProcessingType: 'static {
    type Comp: Component;
    fn get_component() -> Self::Comp;
    fn matches(ext: String, config: &Res<Config>) -> bool;
    fn system(query: Query<&FileQueuedForProcessing, With<Self::Comp>>, config: Res<Config>);

    fn register(app: &mut App) {
        app.add_systems(Update, Self::system);
    }
}

#[derive(Component, Debug)]
pub struct RefreshTimer(pub Timer);

pub fn check_for_stale_files(
    mut timer_query: Query<&mut RefreshTimer>,
    mut commands: Commands,
    time: Res<Time>,
    config: Res<Config>,
) {
    let mut timer = timer_query.single_mut();
    timer.0.tick(time.delta());
    if !timer.0.finished() {
        return;
    }

    for entry_result in WalkDir::new(Path::new("assets-dev"))
        .follow_links(true)
        .sort_by_file_name()
    {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) => {
                // handle IO errors
                eprintln!("Error encountered while checking for stale files: {:}", err);
                continue;
            }
        };
        if entry.file_type().is_dir() {
            // replicate directory structure
            // TODO: would be nice to be able to omit empty dirs.

            let _ = fs::create_dir_all(entry.path());
            continue;
        }

        let source_path = Path::new("assets-dev").join(entry.clone().into_path());
        let dest_path = Path::new("assets").join(entry.into_path());
        if is_stale(&source_path, &dest_path) {
            if queue_file(&mut commands, source_path.clone(), dest_path, &config) {
                println!("Stale File: {}", source_path.display());
            }
        }
    }
}

fn is_stale(source: &PathBuf, dest: &PathBuf) -> bool {
    // get metadata, defaulting to mark as stale if it cannot be found
    // no need to check if the paths exist since that's built in to the metadata error
    let Ok(meta_source) = fs::metadata(source) else {
        return true;
    };
    let Ok(meta_dest) = fs::metadata(dest) else {
        return true;
    };
    let time_source = match meta_source.modified() {
        Ok(time) => Some(time),
        Err(_) => match meta_source.accessed() {
            Ok(time) => Some(time),
            Err(_) => None,
        },
    };
    let time_dest = match meta_dest.modified() {
        Ok(time) => Some(time),
        Err(_) => match meta_dest.accessed() {
            Ok(time) => Some(time),
            Err(_) => None,
        },
    };
    if time_source.is_none() || time_dest.is_none() {
        eprintln!("Your system does not support some of the basic file operations required for this app to work. Honestly I have no clue how we got here. Error referring to: {}", source.display());
        return false;
    }
    // unwrapping should technically be safe at this point.
    time_source.unwrap().cmp(time_dest.unwrap());

    false
}
fn queue_file(
    commands: &mut Commands,
    source: PathBuf,
    dest: PathBuf,
    config: &Res<Config>,
) -> bool {
    let Some(file_ext_os) = source.extension() else {
        return false;
    };
    let Some(file_ext) = file_ext_os
        .to_ascii_lowercase()
        .to_str()
        .and_then(|s| Some(s.to_string()))
    else {
        return false;
    };
    let mut ec = commands.spawn(FileQueuedForProcessing { source, dest });
    if ProcessingRaw::matches(file_ext, config) {
        ec.insert(ProcessingRaw::get_component());
        return true;
    }
    false
}
