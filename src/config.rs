use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_profile: String,
    pub default_printer: String,
    pub import: ImportConfig,
    pub profiles: Vec<Profile>,
    pub layout: LayoutConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportConfig {
    pub default_action: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub offset_stamp_x: f32,
    pub offset_stamp_y: f32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct LayoutConfig {
    pub grid_cols: usize,
    pub grid_rows_max: usize,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_profile: "DL".to_string(),
            default_printer: "lpr".to_string(),
            import: ImportConfig {
                default_action: "copy".to_string(),
            },
            profiles: vec![
                Profile {
                    name: "DL".to_string(),
                    width: 220.0,
                    height: 110.0,
                    offset_stamp_x: 180.0,
                    offset_stamp_y: 10.0,
                },
                Profile {
                    name: "C6".to_string(),
                    width: 162.0,
                    height: 114.0,
                    offset_stamp_x: 130.0,
                    offset_stamp_y: 10.0,
                },
            ],
            layout: LayoutConfig {
                grid_cols: 4,
                grid_rows_max: 8,
            },
        }
    }
}

pub fn load() -> crate::error::Result<Config> {
    let config: Config = confy::load("postamt", None)?;
    Ok(config)
}

pub fn show_profiles(config: &Config) -> crate::error::Result<()> {
    println!("Available profiles:");
    for profile in &config.profiles {
        let marker = if profile.name == config.default_profile {
            " (default)"
        } else {
            ""
        };
        println!("  {} - {}x{}mm{}", profile.name, profile.width, profile.height, marker);
    }
    Ok(())
}

pub fn get_profile<'a>(config: &'a Config, name: &str) -> Option<&'a Profile> {
    config.profiles.iter().find(|p| p.name == name)
}
