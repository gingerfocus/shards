// pub mod line;
// mod abbrs;

struct Config {}

use std::path::PathBuf;

use crate::prelude::*;
use crate::RushiArgs;

pub struct ConfigPaths {
    user: Option<PathBuf>,
    sys: Option<PathBuf>,
}

impl ConfigPaths {
    pub fn new(args: &RushiArgs) -> ConfigPaths {
        if args.no_config {
            ConfigPaths {
                user: None,
                sys: None,
            }
        } else {
            // let Some(p) = args.custom_config

            let xdg = xdg::BaseDirectories::new().unwrap();
            let user = xdg.create_config_directory(PathBuf::from("stardust")).ok();
            let sys = PathBuf::from("/etc/stardust");

            ConfigPaths {
                user: user,
                sys: Some(sys),
            }
        }
    }

    /// Parse init files. exec_path is the path of fish executable as determined by argv[0].
    pub fn source(self, interpreter: &Interpreter, env: &mut UserState) {
        if let Some(user) = self.user {
            log::info!("sourcing user config");
            Self::source_config_in_directory(user, env, interpreter);
        }
        if let Some(sys) = self.sys {
            log::info!("sourcing system config");
            Self::source_config_in_directory(sys, env, interpreter);
        }
    }

    /// Source the file config.rsi or config.zigi in the given directory.
    /// If the config file doesn't exist or isn't readable silently return.
    fn source_config_in_directory(path: PathBuf, env: &mut UserState, parser: &Interpreter) {
        // We need to get the configuration directory before we can source the user configuration file.
        // If path_get_config returns false then we have no configuration directory and no custom config
        // to load.

        let configs = Self::get_config_files_in_dir(path);
        match configs {
            Ok(files) => {}
            Err(e) => {}
        }
        // let paths = get_config_paths();
        // if let Some(path) = paths.data {
        //     for path in paths {
        //         source_config_in_directory(path, env);
        //     }
        // }

        // parser.parse(env);
    }

    fn get_config_files_in_dir(dir: PathBuf) -> Result<Vec<PathBuf>> {
        let mut rust = dir.clone();
        rust.push("config.rsi");
        let mut zig = dir;
        zig.push("config.zigi");

        let mut sourcable_files = Vec::new();
        if rust.exists() {
            log::info!("found rsi config: {:?}", rust);
            sourcable_files.push(rust)
        } else {
            log::warn!("not sourcing {:?} (Does not exist)", rust);
        }
        if zig.exists() {
            log::info!("found rsi config: {:?}", zig);
            sourcable_files.push(zig)
        }

        Ok(sourcable_files)
    }
}
