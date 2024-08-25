use std::fs;

use bevy::prelude::*;

use crate::processing::{FileQueuedForProcessing, ProcessingType};

#[derive(Component)]
pub struct FileRaw;

pub struct ProcessingRaw;

impl ProcessingType for ProcessingRaw {
    type Comp = FileRaw;

    fn get_component() -> Self::Comp {
        FileRaw
    }

    fn matches(ext: &String, config: &Res<crate::config::Config>) -> bool {
        let valid_ext = config.extensions.raw.clone();
        valid_ext.contains(&ext)
    }

    fn system(
        query: Query<(Entity, &FileQueuedForProcessing), With<Self::Comp>>,
        _: Res<crate::config::Config>, // config needed for other processing types. Not here
        mut commands: Commands,
    ) {
        for (e, entry) in query.iter() {
            match fs::copy(entry.source.clone(), entry.dest.clone()) {
                Ok(_) => {
                    let time = crate::processing::get_human_duration(entry.queue_time.elapsed());
                    info!("RAW => {} -- {}", entry.dest.display(), time);
                    commands.entity(e).despawn_recursive()
                }
                Err(err) => panic!(
                    "Failed to copy raw file to assets dir. File data {:#?}. Error: {}",
                    entry, err
                ),
            }
        }
    }
}
