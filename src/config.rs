use bevy::prelude::Resource;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Resource, Debug, Clone)]
pub struct Config {
    pub file_watching_rate_seconds: f64,
    pub extensions: Extensions,
    pub meshes: MeshConfigs,
    pub textures: TextureConfigs,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Extensions {
    pub raw: Vec<String>,
    pub texture: Vec<String>,
    pub mesh: Vec<String>,
    pub audio: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MeshConfigs {
    pub use_meshlets: bool,
    pub storage: MeshStorage,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum MeshStorage {
    Glb,
    Gltf,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct TextureConfigs {
    pub filter: TextureFilter,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum TextureFilter {
    Nearest,
    Linear,
}
#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct AudioConfigs {}

impl Default for Config {
    fn default() -> Self {
        Self {
            file_watching_rate_seconds: 0.3,
            extensions: Extensions {
                raw: vec![],
                texture: vec!["jpg".into(), "png".into()],
                mesh: vec!["glb".into(), "gltf".into()],
                audio: vec!["ogg".into(), "wav".into()],
            },
            // replicate_ext: vec![".*^[jpg|png|glb|gltf|wav|mp3]".into()],
            // ext_mesh: vec!["glb".into(), "gltf".into()],
            // regex_texture: vec![],
            // regex_audio: vec![],
            meshes: MeshConfigs {
                use_meshlets: false,
                storage: MeshStorage::Glb,
            },
            textures: TextureConfigs {
                filter: TextureFilter::Linear,
            },
        }
    }
}

pub fn load_config(text: &str) -> Option<Config> {
    if let Ok(config) = toml::from_str(text) {
        return Some(config);
    }
    None
}

pub fn get_default_configuration_text() -> Option<String> {
    if let Ok(text) = toml::to_string_pretty(&Config::default()) {
        return Some(text);
    }
    None
}
