use std::path::Path;
use std::sync::LazyLock;

pub fn config() -> &'static Path {
    PROJECT
        .as_ref()
        .map(directories::ProjectDirs::config_dir)
        .unwrap_or(Path::new("./config"))
}

static PROJECT: LazyLock<Option<directories::ProjectDirs>> =
    LazyLock::new(|| directories::ProjectDirs::from("rs.convo", "", "convo"));
