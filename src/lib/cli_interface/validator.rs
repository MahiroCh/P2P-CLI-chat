const VALID_LOG_LEVELS: &[&str] = &["error", "warn", "info", "debug"];

pub fn parse_log_level(level: &str) -> Result<String, String> {
  if VALID_LOG_LEVELS.contains(&level) {
    Ok(level.to_owned())
  } else {
    Err(format!(
      "invalid log level '{level}'. Expected one of: {}",
      VALID_LOG_LEVELS.join(", ")
    ))
  }
}
