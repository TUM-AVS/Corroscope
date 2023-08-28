use bevy::prelude::Resource;
// use clap::Parser;

/// Interactive CommonRoad scenario inspector
#[derive(Debug, Resource)]
// #[command(author, version, about, long_about = None)]
pub struct Args {
    /// Path to the reactive planner logs directory
    // #[arg(long)]
    pub logs: std::path::PathBuf,
}

impl Args {
    pub(crate) fn parse() -> Self {
        Self {
            logs: std::env::args().nth(1).unwrap().into(),
        }
    }
}

/*

    /// Path to the scenario file
    #[arg(long)]
    pub scenario: std::path::PathBuf,

    /// Path to the reference path file (reference_path.json)
    #[arg(long)]
    pub reference_path: std::path::PathBuf,
*/
