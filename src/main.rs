use std::{
    fs::{self},
    path::Path,
};

use bevy::{
    log::{Level, LogPlugin},
    prelude::*,
};
use clap::Parser;
use config::Config;
use mesh::ProcessingMesh;
use processing::{ProcessingType, RefreshTimer};
use raw::ProcessingRaw;

mod audio;
mod config;
mod mesh;
mod processing;
mod raw;
mod texture;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Cli {
    #[arg(short, long, value_name = "BOOL", action=clap::ArgAction::SetTrue)]
    oneshot: Option<bool>,
    #[arg(short, long, value_name = "BOOL", action=clap::ArgAction::SetTrue)]
    verbose: Option<bool>,
}

fn main() {
    let cli = Cli::parse();
    let config = load_configuration().unwrap_or_default();
    let mut app = App::new();
    app.add_plugins(MinimalPlugins)
        .add_plugins(LogPlugin {
            level: if cli.verbose.unwrap_or_default() {
                Level::DEBUG
            } else {
                Level::INFO
            },
            ..default()
        })
        .insert_resource(config)
        .add_systems(Startup, initialize)
        .add_systems(Update, processing::check_for_stale_files);
    ProcessingRaw::register(&mut app);
    ProcessingMesh::register(&mut app);

    let oneshot = cli.oneshot.unwrap_or(false);

    if oneshot {
        app.update();
    } else {
        app.run();
    }
    debug!("Handled CLI data {:?}", cli);
}
fn initialize(mut commands: Commands, config: Res<Config>) {
    commands.spawn(RefreshTimer(Timer::from_seconds(
        config.file_watching_rate_seconds as f32,
        TimerMode::Repeating,
    )));
}

fn load_configuration() -> Option<Config> {
    let _ = fs::create_dir(Path::new("assets-dev")); // ignore errors
    let config_path = Path::new("assets-dev").join("config.toml");

    let Ok(file_data) = fs::read(config_path.clone()) else {
        let Some(config_text) = config::get_default_configuration_text() else {
            return None;
        };

        let _ = fs::write(config_path, config_text);
        return None;
    };
    let Ok(file_text) = String::from_utf8(file_data) else {
        return None;
    };

    if let Some(config) = config::load_config(file_text.as_str()) {
        return Some(config);
    } else {
        error!("Configuration appears to be corrupted");
    }
    None
}
