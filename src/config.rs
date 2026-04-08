use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub default_profile: String,
    pub default_printer: Option<String>,
    pub import: ImportConfig,
    #[serde(skip)]
    pub profiles: HashMap<String, Profile>,
}

// Intermediate struct for loading from TOML
#[derive(Debug, Serialize, Deserialize)]
struct ConfigFile {
    default_profile: String,
    default_printer: Option<String>,
    import: ImportConfig,
    #[serde(default)]
    profiles: HashMap<String, Profile>,
}

impl Default for ConfigFile {
    fn default() -> Self {
        ConfigFile {
            default_profile: "DL".to_string(),
            default_printer: None,
            import: ImportConfig {
                default_action: ImportAction::Copy,
            },
            profiles: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ImportAction {
    Copy,
    Move,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImportConfig {
    pub default_action: ImportAction,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub width: f32,
    pub height: f32,
    pub offset_stamp_x: f32,
    pub offset_stamp_y: f32,
}

fn default_profiles() -> HashMap<String, Profile> {
    let mut profiles = HashMap::new();
    profiles.insert(
        "DL".to_string(),
        Profile {
            width: 220.0,
            height: 110.0,
            offset_stamp_x: 170.0,
            offset_stamp_y: 8.0,
        },
    );
    profiles.insert(
        "C6".to_string(),
        Profile {
            width: 162.0,
            height: 114.0,
            offset_stamp_x: 130.0,
            offset_stamp_y: 10.0,
        },
    );
    profiles
}

impl Default for Config {
    fn default() -> Self {
        Config {
            default_profile: "DL".to_string(),
            default_printer: None,
            import: ImportConfig {
                default_action: ImportAction::Copy,
            },
            profiles: default_profiles(),
        }
    }
}

pub fn load() -> crate::error::Result<Config> {
    // Use standard XDG config path: ~/.config/postamt/config.toml
    let config_file = if let Some(config_dir_base) = dirs::config_dir() {
        let config_dir = config_dir_base.join("postamt");
        let config_path = config_dir.join("config.toml");

        if config_path.exists() {
            // Read and parse existing config
            let contents = std::fs::read_to_string(&config_path)?;
            toml::from_str(&contents)
                .map_err(|e| crate::error::Error::Custom(format!("Config parse error: {}", e)))?
        } else {
            // No config file, use defaults
            ConfigFile::default()
        }
    } else {
        // No config directory found, use defaults
        ConfigFile::default()
    };

    // Start with default profiles
    let mut profiles = default_profiles();

    // Extend/override with user profiles from config file
    profiles.extend(config_file.profiles);

    // Build final Config
    Ok(Config {
        default_profile: config_file.default_profile,
        default_printer: config_file.default_printer,
        import: config_file.import,
        profiles,
    })
}

pub fn show_profiles(config: &Config) -> crate::error::Result<()> {
    println!("Available profiles:");
    let mut profile_names: Vec<_> = config.profiles.keys().cloned().collect();
    profile_names.sort();

    for name in profile_names {
        let profile = &config.profiles[&name];
        let marker = if name == config.default_profile {
            " (default)"
        } else {
            ""
        };
        println!(
            "  {} - {}x{}mm{}",
            name, profile.width, profile.height, marker
        );
    }
    Ok(())
}

pub fn get_profile<'a>(config: &'a Config, name: &str) -> Option<&'a Profile> {
    config.profiles.get(name)
}

fn get_config_path() -> crate::error::Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        crate::error::Error::Custom("Could not determine config directory".to_string())
    })?;
    Ok(config_dir.join("postamt").join("config.toml"))
}

fn ensure_config_dir() -> crate::error::Result<()> {
    let config_dir = dirs::config_dir().ok_or_else(|| {
        crate::error::Error::Custom("Could not determine config directory".to_string())
    })?;
    std::fs::create_dir_all(config_dir.join("postamt"))?;
    Ok(())
}

fn load_config_file() -> crate::error::Result<toml::Value> {
    let config_path = get_config_path()?;
    if config_path.exists() {
        let contents = std::fs::read_to_string(&config_path)?;
        toml::from_str(&contents)
            .map_err(|e| crate::error::Error::Custom(format!("Config parse error: {}", e)))
    } else {
        Ok(toml::Value::Table(toml::map::Map::new()))
    }
}

fn save_config_file(value: &toml::Value) -> crate::error::Result<()> {
    ensure_config_dir()?;
    let config_path = get_config_path()?;
    let contents = toml::to_string_pretty(value)
        .map_err(|e| crate::error::Error::Custom(format!("Config serialize error: {}", e)))?;
    std::fs::write(&config_path, contents)?;
    Ok(())
}

pub fn handle_config_command(key: Option<&str>, value: Option<&str>) -> crate::error::Result<()> {
    let mut config = load_config_file()?;

    match (key, value) {
        // No args: show whole config
        (None, None) => {
            let output = toml::to_string_pretty(&config).map_err(|e| {
                crate::error::Error::Custom(format!("Config serialize error: {}", e))
            })?;
            if output.trim().is_empty() {
                println!("# No config set. Config file: {:?}", get_config_path()?);
            } else {
                println!("{}", output);
            }
        }
        // Key only: get value
        (Some(key), None) => {
            let parts: Vec<&str> = key.split('.').collect();
            let val = get_nested_value(&config, &parts);
            match val {
                Some(v) => println!("{}", format_value(v)),
                None => println!("# Not set"),
            }
        }
        // Key + value: set value
        (Some(key), Some(value)) => {
            let parts: Vec<&str> = key.split('.').collect();
            let parsed_value = parse_value(value);
            set_nested_value(&mut config, &parts, parsed_value)?;
            save_config_file(&config)?;
            println!("Set {} = {}", key, value);
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn get_nested_value<'a>(config: &'a toml::Value, parts: &[&str]) -> Option<&'a toml::Value> {
    let mut current = config;
    for part in parts {
        current = current.get(part)?;
    }
    Some(current)
}

fn set_nested_value(
    config: &mut toml::Value,
    parts: &[&str],
    value: toml::Value,
) -> crate::error::Result<()> {
    if parts.is_empty() {
        return Err(crate::error::Error::Custom("Empty key".to_string()));
    }

    let mut current = config;
    for part in &parts[..parts.len() - 1] {
        if !current
            .as_table()
            .map(|t| t.contains_key(*part))
            .unwrap_or(false)
        {
            current
                .as_table_mut()
                .unwrap()
                .insert(part.to_string(), toml::Value::Table(toml::map::Map::new()));
        }
        current = current.get_mut(part).unwrap();
    }

    let last_key = parts.last().unwrap();
    current
        .as_table_mut()
        .ok_or_else(|| crate::error::Error::Custom("Cannot set value on non-table".to_string()))?
        .insert(last_key.to_string(), value);

    Ok(())
}

fn parse_value(s: &str) -> toml::Value {
    // Try integer
    if let Ok(i) = s.parse::<i64>() {
        return toml::Value::Integer(i);
    }
    // Try float
    if let Ok(f) = s.parse::<f64>() {
        return toml::Value::Float(f);
    }
    // Try boolean
    if s == "true" {
        return toml::Value::Boolean(true);
    }
    if s == "false" {
        return toml::Value::Boolean(false);
    }
    // Default to string
    toml::Value::String(s.to_string())
}

fn format_value(v: &toml::Value) -> String {
    match v {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(i) => i.to_string(),
        toml::Value::Float(f) => f.to_string(),
        toml::Value::Boolean(b) => b.to_string(),
        toml::Value::Array(_) | toml::Value::Table(_) => {
            toml::to_string_pretty(v).unwrap_or_else(|_| format!("{:?}", v))
        }
        toml::Value::Datetime(d) => d.to_string(),
    }
}
