use localcode_core::config::Config;
use crate::rendering;

pub fn run_config(set: Option<&str>, get: Option<&str>, show: bool) {
    let mut config = Config::load().unwrap_or_default();

    if let Some(set_str) = set {
        if let Some((key, value)) = set_str.split_once('=') {
            match key.trim() {
                "default_provider" => config.default_provider = value.trim().to_string(),
                "openai.api_key" => config.providers.openai.api_key = value.trim().to_string(),
                "openai.model" => config.providers.openai.model = value.trim().to_string(),
                "openai.base_url" => config.providers.openai.base_url = value.trim().to_string(),
                "anthropic.api_key" => config.providers.anthropic.api_key = value.trim().to_string(),
                "anthropic.model" => config.providers.anthropic.model = value.trim().to_string(),
                "local.server_url" => config.providers.local.server_url = value.trim().to_string(),
                "local.model_path" => config.providers.local.model_path = value.trim().to_string(),
                _ => {
                    rendering::print_error(&format!("Unknown config key: {}", key));
                    return;
                }
            }
            match config.save() {
                Ok(_) => rendering::print_success(&format!("Set {} = {}", key, value)),
                Err(e) => rendering::print_error(&format!("Failed to save config: {}", e)),
            }
        } else {
            rendering::print_error("Format: --set key=value");
        }
        return;
    }

    if let Some(key) = get {
        let value = match key {
            "default_provider" => config.default_provider.clone(),
            "openai.api_key" => mask_key(&config.providers.openai.api_key),
            "openai.model" => config.providers.openai.model.clone(),
            "anthropic.api_key" => mask_key(&config.providers.anthropic.api_key),
            "anthropic.model" => config.providers.anthropic.model.clone(),
            _ => {
                rendering::print_error(&format!("Unknown config key: {}", key));
                return;
            }
        };
        println!("{} = {}", key, value);
        return;
    }

    if show {
        println!("Configuration ({})", Config::config_path().display());
        println!("  default_provider: {}", config.default_provider);
        println!("  openai.api_key: {}", mask_key(&config.providers.openai.api_key));
        println!("  openai.model: {}", config.providers.openai.model);
        println!("  openai.base_url: {}", config.providers.openai.base_url);
        println!("  anthropic.api_key: {}", mask_key(&config.providers.anthropic.api_key));
        println!("  anthropic.model: {}", config.providers.anthropic.model);
        println!("  local.server_url: {}", config.providers.local.server_url);
        return;
    }

    // Default: show usage
    println!("Usage:");
    println!("  localcode config --show");
    println!("  localcode config --set key=value");
    println!("  localcode config --get key");
    println!();
    println!("Keys: default_provider, openai.api_key, openai.model,");
    println!("  openai.base_url, anthropic.api_key, anthropic.model,");
    println!("  local.server_url, local.model_path");
}

fn mask_key(key: &str) -> String {
    if key.is_empty() {
        "(not set)".to_string()
    } else if key.len() > 8 {
        format!("{}...{}", &key[..4], &key[key.len()-4..])
    } else {
        "****".to_string()
    }
}
