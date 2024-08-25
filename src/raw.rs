use bevy::prelude::*;

use crate::processing::ProcessingType;

#[derive(Component)]
pub struct FileRaw;

pub struct ProcessingRaw;

impl ProcessingType for ProcessingRaw {
    type Comp = FileRaw;

    fn get_component() -> Self::Comp {
        FileRaw
    }

    fn matches(ext: String, config: &Res<crate::config::Config>) -> bool {
        let valid_ext = config.extensions.raw.clone();
        valid_ext.contains(&ext)
    }

    fn system(
        query: Query<&crate::processing::FileQueuedForProcessing, With<Self::Comp>>,
        config: Res<crate::config::Config>,
    ) {
        todo!()
    }
}
