use std::fs::read_to_string;
use toml::from_str;

// -----------------------------------------------------------------------------
// Load TOML configurations from file
//
//   Arguments:
//     - full pathname of configuration file (TOML)
//   Returns:
//     - a completed Config struct
//
pub fn load_toml_config<T: for<'de> serde::Deserialize<'de>>(path: &str) -> Result<T, String> {
  //
  // Read TOML configuration file
  //
  let toml_content = read_to_string(path).map_err(|err| format!("Failed to read TOML file: {}", err))?;

  // Parse TOML file into Config struct
  //
  let config = from_str(&toml_content).map_err(|err| format!("Failed to parse TOML file: {}", err))?;

  Ok(config)
}
