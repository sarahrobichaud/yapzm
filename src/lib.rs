use std::path::PathBuf;

pub mod cli;

pub mod config;

pub mod platform;

pub mod steam;

pub struct ClientPaths {
    pub root: PathBuf,
    pub libraries: Vec<PathBuf>,
}
