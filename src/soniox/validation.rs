use crate::errors::SonioxWindowsErrors;
use crate::types::settings::SettingsApp;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Model {
    id: String,
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<Model>,
}

pub fn validate_model(settings: &SettingsApp) -> Result<(), SonioxWindowsErrors> {
    log::info!("Validating model '{}'...", settings.model());

    let client = reqwest::blocking::Client::new();
    let response = client
        .get("https://api.soniox.com/v1/models")
        .header("Authorization", format!("Bearer {}", settings.api_key()))
        .send()
        .map_err(|e| SonioxWindowsErrors::Internal(e.to_string()))?;

    let status = response.status();
    if !status.is_success() {
        return Err(SonioxWindowsErrors::Internal(format!(
            "Failed to fetch models: {} (Status: {})",
            response.text().unwrap_or_default(),
            status
        )));
    }

    let models_resp: ModelsResponse = response.json().map_err(|e| {
        SonioxWindowsErrors::Internal(format!("Failed to parse models response: {}", e))
    })?;

    let configured_model = settings.model();
    let exists = models_resp.models.iter().any(|m| m.id == configured_model);

    if exists {
        log::info!("Model '{}' is valid.", configured_model);
        Ok(())
    } else {
        let available: Vec<&str> = models_resp
            .models
            .iter()
            .map(|m| m.id.as_str())
            .filter(|id| id.contains("-rt-"))
            .collect();
        log::error!("Invalid model configured: {}. Available (RT): {:?}", configured_model, available);
        Err(SonioxWindowsErrors::Internal(format!(
            "Invalid model configured: '{}'.\nAvailable Real-Time models: {}",
            configured_model,
            available.join(", ")
        )))
    }
}
