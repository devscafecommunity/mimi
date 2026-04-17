use super::defaults::Config;
use crate::cli::error::{CliError, CliResult};
use std::path::PathBuf;

pub struct ConfigLoader;

impl ConfigLoader {
    pub fn load(_config_path: Option<PathBuf>) -> CliResult<Config> {
        Ok(Config::default())
    }

    pub fn load_from_file(_path: &PathBuf) -> CliResult<Config> {
        Ok(Config::default())
    }

    pub fn save(_config: &Config, _path: &PathBuf) -> CliResult<()> {
        Ok(())
    }

    pub fn get_value(_config: &Config, _key: &str) -> CliResult<String> {
        Ok("".to_string())
    }

    pub fn set_value(_config: &mut Config, _key: &str, _value: &str) -> CliResult<()> {
        Ok(())
    }
}
