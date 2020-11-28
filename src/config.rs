use std::fs;
use std::io;
use std::path::PathBuf;

use app_dirs::{data_root, AppDataType, AppDirsError};

use crate::error::Error;
use crate::sf::{self, EntityField};

/// The app configuration.
#[derive(Debug)]
pub struct Config {
    /// Additional fields that must be included in the output.
    pub additional_fields: Vec<EntityField>,
    /// Fields that must be used when searching (values must be strings).
    pub search_fields: Vec<EntityField>,
}

impl Config {
    /// Open the configuration file with the default editor.
    /// Return an error based on the editor's exit code.
    pub fn edit() -> Result<(), Error> {
        match config_path() {
            Ok(path) => {
                // Open the configuration from the path, or use a default empty one.
                let conf = match FileConf::from_path(&path) {
                    Ok(conf) => conf,
                    Err(_) => FileConf::empty(),
                };

                // Open the default editor and retrieve the edited configuraton.
                let contents = match edit::edit(toml::to_string(&conf).unwrap()) {
                    Ok(s) => s,
                    Err(err) => {
                        return Err(Error {
                            message: format!("cannot open default editor: {}", err),
                        })
                    }
                };

                // Validate the new configuration.
                match toml::from_str::<FileConf>(&contents) {
                    Ok(conf) => conf.to_config()?,
                    Err(err) => {
                        return Err(Error {
                            message: format!("cannot deserialize provided config: {}", err),
                        })
                    }
                };

                // Save the new configuration to file.
                match write_file(&path, &contents) {
                    Ok(_) => Ok(()),
                    Err(err) => Err(Error {
                        message: format!("cannot write config: {}", err),
                    }),
                }
            }
            Err(err) => Err(Error {
                message: format!("cannot get config file path: {}", err),
            }),
        }
    }

    /// Parse the configuration file and returns a `Config`.
    pub fn parse() -> Result<Config, Error> {
        match config_path() {
            Ok(path) => {
                // Open the configuration from the path, or use a default empty one.
                let conf = match FileConf::from_path(&path) {
                    Ok(conf) => conf,
                    Err(_) => FileConf::empty(),
                };
                conf.to_config()
            }
            Err(err) => Err(Error {
                message: format!("cannot get config file path: {}", err),
            }),
        }
    }
}

/// Return the path to the configuration file.
/// Both the file and the directory it lives in might not exist.
fn config_path() -> Result<PathBuf, AppDirsError> {
    let mut p = data_root(AppDataType::UserConfig)?;
    p.push("sfind");
    p.push("config.toml");
    Ok(p)
}

/// Write the given contents in the file at the given path.
/// Create directories if required.
fn write_file(path: &PathBuf, contents: &str) -> Result<(), io::Error> {
    fs::create_dir_all(path.parent().unwrap())?;
    fs::write(path, contents)?;
    Ok(())
}

/// The raw configuration for the app.
#[derive(serde::Deserialize, serde::Serialize, Debug)]
struct FileConf {
    pub fields: Vec<String>,
    pub search: Vec<String>,
}

impl FileConf {
    /// Return an empty configuration.
    fn empty() -> Self {
        Self {
            fields: vec![],
            search: vec![],
        }
    }

    /// Return the configuration stored in the file at the given path.
    fn from_path(path: &PathBuf) -> Result<Self, io::Error> {
        let contents = fs::read_to_string(path)?;
        let conf: FileConf = toml::from_str(&contents)?;
        Ok(conf)
    }

    /// Create a `Config` from the `FileConf`.
    fn to_config(&self) -> Result<Config, Error> {
        let fields: Result<Vec<EntityField>, sf::Error> = self
            .fields
            .iter()
            .map(|f| f.parse::<EntityField>())
            .collect();
        let search: Result<Vec<EntityField>, sf::Error> = self
            .search
            .iter()
            .map(|f| f.parse::<EntityField>())
            .collect();
        let additional_fields = fields?;
        let search_fields = search?;
        Ok(Config {
            additional_fields,
            search_fields,
        })
    }
}

// TODO(frankban): test this module.
