use std::{
    cmp::Ordering,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

use crate::{
    config::Config,
    mesh::ProcessingMesh,
    raw::{self, ProcessingRaw},
};
use bevy::prelude::*;
use humantime::format_duration;
use walkdir::WalkDir;

#[derive(Resource)]
pub struct UnprocessedFiles(pub usize);

/// The core component that links an entity to a specific file in the staging directory
#[derive(Component, Debug)]
pub struct FileQueuedForProcessing {
    pub source: PathBuf,
    pub dest: PathBuf,
    pub queue_time: Instant,
}

/// The core trait for processing information.
/// Less for dealing with processing types in generic form but for ensuring each processing type meets common constraints
pub trait ProcessingType: 'static {
    type Comp: Component;
    fn get_component() -> Self::Comp;
    fn matches(ext: &String, config: &Res<Config>) -> bool;
    fn get_destination(source: &PathBuf) -> Option<PathBuf>;
    fn system(
        query: Query<(Entity, &FileQueuedForProcessing), With<Self::Comp>>,
        config: Res<Config>,
        commands: Commands,
    );

    fn register(app: &mut App) {
        app.add_systems(Update, Self::system);
    }
}

pub struct AssetProcessing;

impl AssetProcessing {
    // I thought a type to encapsulate the fns would be useful, but right now there's just the one func. Shame about that

    fn get_destination(source: &PathBuf) -> Option<PathBuf> {
        if let Some(path) = raw::ProcessingRaw::get_destination(source) {
            return Some(path);
        }
        None
    }
}

#[derive(Component, Debug)]
pub struct RefreshTimer(pub Timer);

pub fn check_for_stale_files(
    mut timer_query: Query<&mut RefreshTimer>,
    currently_queued: Query<&FileQueuedForProcessing>,
    mut commands: Commands,
    mut unprocessed: ResMut<UnprocessedFiles>,
    time: Res<Time>,
    config: Res<Config>,
) {
    let mut timer = timer_query.single_mut();
    timer.0.tick(time.delta());
    if !timer.0.finished() {
        return;
    }
    let currently_queued_paths = currently_queued
        .iter()
        .map(|comp| comp.source.clone())
        .collect::<Vec<_>>();

    let mut count: usize = 0;
    let mut unhandled_files = Vec::<PathBuf>::new();

    for entry_result in WalkDir::new(Path::new("assets-dev"))
        .follow_links(true)
        .sort_by_file_name()
    {
        let entry = match entry_result {
            Ok(e) => e,
            Err(err) => {
                // handle IO errors
                error!("Error encountered while checking for stale files: {:}", err);
                continue;
            }
        };
        if entry.path() == Path::new("assets-dev").join("config.toml") {
            continue;
        }
        let Ok(entry_path) = entry.path().strip_prefix(Path::new("assets-dev")) else {
            error!("Failed to strip prefix 'assets-dev' from source file path. This likely means we somehow got in the wrong folder!!!");
            continue;
        };
        let source_path = Path::new("assets-dev").join(entry_path);
        let Some(dest_path) = AssetProcessing::get_destination(&source_path) else {
            continue;
        };

        if currently_queued_paths.contains(&source_path) {
            // skip already queued paths.
            continue;
        }
        if entry.file_type().is_dir() {
            // replicate directory structure
            // TODO: would be nice to be able to omit empty dirs.

            let _ = fs::create_dir_all(dest_path);
            continue;
        }

        if is_stale(&source_path, &dest_path) {
            if queue_file(&mut commands, source_path.clone(), dest_path, &config) {
                count += 1;
                debug!("Queued for processing: {}", source_path.display());
            } else {
                unhandled_files.push(source_path);
            }
        }
    }
    unprocessed.0 = count;
    if count > 0 {
        let total = count + currently_queued_paths.len();
        debug!(
            "Queued {} files for processing. {} file already queued. {} total files in processing",
            count,
            currently_queued_paths.len(),
            total
        );
        debug!("Previously queued paths: {:#?}", currently_queued_paths);
    }
    if !unhandled_files.is_empty() {
        let display_files = unhandled_files
            .iter()
            .map(|path| path.display())
            .collect::<Vec<_>>();
        debug!("Unhandled Files: {:#?}", display_files);
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
        error_once!("Your system does not support some of the basic file operations required for this app to work. Honestly I have no clue how we got here. Error referring to: {}", source.display());
        return false;
    }
    // unwrapping should technically be safe at this point.
    time_source.unwrap().cmp(&time_dest.unwrap()) == Ordering::Greater
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

    let fqfp = FileQueuedForProcessing {
        source,
        dest,
        queue_time: Instant::now(),
    };
    if ProcessingRaw::matches(&file_ext, config) {
        commands.spawn((fqfp, ProcessingRaw::get_component()));
        return true;
    }
    if ProcessingMesh::matches(&file_ext, config) {
        commands.spawn((fqfp, ProcessingMesh::get_component()));
        return true;
    }
    false
}

pub fn get_human_duration(duration: Duration) -> String {
    format_duration(duration).to_string()
}
