use std::{
    fs,
    path::{Path, PathBuf},
    pin::pin,
};

use bevy::{
    gltf::Gltf,
    log::{error, info},
    prelude::{Component, DespawnRecursiveExt, Res},
    tasks::futures_lite::stream::{self, block_on},
};
use gltf_kun::{
    extensions::DefaultExtensions,
    graph::Graph,
    io::format::{
        glb::{GlbExport, GlbFormat, GlbImport},
        gltf::{self, GltfFormat},
    },
};

use crate::{config::Config, processing::ProcessingType};

#[derive(Component)]
pub struct FileMesh;

/// A component to mark mesh files that cannot be processed until the necessary textures are completed
#[derive(Component, Debug)]
struct FileMeshAwaitingTextures {
    textures: Vec<SourceDestPair>,
}

#[derive(Debug)]
struct SourceDestPair {
    source: PathBuf,
    destination: PathBuf,
}

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
        query: bevy::prelude::Query<
            (
                bevy::prelude::Entity,
                &crate::processing::FileQueuedForProcessing,
            ),
            bevy::prelude::With<Self::Comp>,
        >,
        config: bevy::prelude::Res<crate::config::Config>,
        mut commands: bevy::prelude::Commands,
    ) {
        for (e, entry) in query.iter() {
            let Some(os_ext) = entry.source.extension() else {
                continue;
            };
            let is_processed = match os_ext.to_str() {
                Some(ext) => match ext.to_ascii_lowercase().as_str() {
                    "glb" => process_gltf_format(
                        &entry.source,
                        &entry.dest,
                        SceneExt::Glb,
                        config.clone(),
                    ),
                    "gltf" => process_gltf_format(
                        &entry.source,
                        &entry.dest,
                        SceneExt::Gltf,
                        config.clone(),
                    ),
                    "glxf" => process_gltf_format(
                        &entry.source,
                        &entry.dest,
                        SceneExt::Glxf,
                        config.clone(),
                    ),
                    _ => false,
                },
                None => false,
            };
            if !is_processed {
                panic!("Failed to find proper processing format for {}. Ensure your configuration is not incorrect. Valid extensions for meshes: [glb, gltf, glxf]", entry.source.display());
            }
            commands.entity(e).despawn_recursive();
        }
    }
}

enum SceneExt {
    Glb,
    Gltf,
    Glxf,
}

fn process_gltf_format(
    source_file: &PathBuf,
    dest_file: &PathBuf,
    format: SceneExt,
    config: Config,
) -> bool {
    let doc = match format {
        SceneExt::Glb => {
            let format = GlbFormat(fs::read(source_file).unwrap_or_default());
            let mut graph = Graph::new();
            let boxed_glb = Box::pin(stream::once_future(GlbImport::<DefaultExtensions>::import(
                &mut graph, format,
            )));
            let mut result_iter = stream::block_on(boxed_glb);
            let Some(res) = result_iter.next() else {
                error!("Failed to load gltf data from future!");
                return false;
            };
            match res {
                Ok(doc) => doc,
                Err(err) => {
                    error!(
                        "GLB Import error on file: {} :: {}",
                        source_file.display(),
                        err
                    );
                    return false;
                }
            }
        }
        SceneExt::Gltf => {
            // let format = GltfFormat {
            //     json: gltf_js,
            //     resources: todo!(),
            // };
            todo!()
        }
        SceneExt::Glxf => todo!(),
    };
    if let Ok(formatted) = GlbExport::<DefaultExtensions>::export(&mut Graph::new(), &doc) {
        let _ = fs::write(dest_file, formatted.0);
        info!("Mesh {} => {}", source_file.display(), dest_file.display());
    }

    true
}
