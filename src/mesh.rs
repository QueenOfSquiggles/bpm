use std::{fs, path::Path};

use bevy::prelude::*;
use bevy_gltf_kun::{
    export::gltf::{GltfExportEvent, GltfExportResult},
    import::gltf::{scene::GltfScene, GltfKun},
};
use gltf_kun::{extensions::DefaultExtensions, io::format::glb::GlbExport};

use crate::{
    config::Config,
    processing::{get_human_duration, FileQueuedForProcessing, ProcessingType},
};

#[derive(Component)]
pub struct FileMesh;

/// A component to mark mesh files that cannot be processed until the necessary textures are completed
// #[derive(Component, Debug)]
// struct FileMeshAwaitingTextures {
//     textures: Vec<SourceDestPair>,
// }

// #[derive(Debug)]
// struct SourceDestPair {
//     source: PathBuf,
//     destination: PathBuf,
// }

pub struct ProcessingMesh;

impl ProcessingType for ProcessingMesh {
    type Comp = FileMesh;

    fn get_component() -> Self::Comp {
        FileMesh
    }

    fn matches(ext: &String, config: &bevy::prelude::Res<crate::config::Config>) -> bool {
        config.extensions.mesh.contains(ext)
    }

    fn get_destination(source: &std::path::PathBuf) -> Option<std::path::PathBuf> {
        let base = source.strip_prefix(Path::new("assets-dev")).ok()?;
        let mut dest_path = Path::new("assets").join(&base);
        dest_path.set_extension("glb"); // not a fan of hard coding that. Is GLB the most efficient?
        Some(dest_path)
    }

    fn system(
        _: Query<(Entity, &FileQueuedForProcessing), With<Self::Comp>>,
        _: Res<Config>,
        _: Commands,
    ) {
    }

    fn register(app: &mut bevy::prelude::App) {
        app.add_systems(Update, (load_gltf_scenes).chain());
    }
}

fn load_gltf_scenes(
    query: Query<&FileQueuedForProcessing, With<FileMesh>>,
    assets: Res<AssetServer>,
    kun_scenes: Res<Assets<GltfKun>>,
    scenes: Res<Assets<GltfScene>>,
    mut export: EventWriter<GltfExportEvent<DefaultExtensions>>,
    mut results: ResMut<Events<GltfExportResult>>,
) {
    for entry in query.iter() {
        let Ok(canonical_path) = entry.source.canonicalize() else {
            error!(
                "Failed to load canonical path for: {}",
                entry.source.display()
            );
            continue;
        };
        debug!("Asset server path to load: {}", canonical_path.display());
        let scene_handle = assets.load::<GltfKun>(canonical_path);
        let gltf = match kun_scenes.get(&scene_handle) {
            Some(a) => a,
            None => {
                error!("Failed to load GltfKun from handle!");
                continue;
            }
        };
        for scene in gltf.scenes.iter() {
            if let Some(scene_asset) = scenes.get(scene) {
                export.send(GltfExportEvent::new(scene_asset.scene.clone()));

                for mut event in results.drain() {
                    let Ok(doc) = event.result else {
                        continue;
                    };
                    let Ok(bytes) = GlbExport::<DefaultExtensions>::export(&mut event.graph, &doc)
                    else {
                        continue;
                    };
                    let _ = fs::write(entry.dest.clone(), bytes.0);
                    let time = entry.queue_time.elapsed();
                    info!(
                        "{} => {} -- {}",
                        entry.source.display(),
                        entry.dest.display(),
                        get_human_duration(time)
                    );
                }
            }
        }
    }
}
