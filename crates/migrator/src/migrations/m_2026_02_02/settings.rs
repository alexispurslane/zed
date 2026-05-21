use anyhow::Result;
use serde_json::Value;

use crate::migrations::migrate_settings;

pub fn move_edit_prediction_provider_to_edit_predictions(value: &mut Value) -> Result<()> {
    migrate_settings(value, &mut migrate_one)
}

fn migrate_one(obj: &mut serde_json::Map<String, Value>) -> Result<()> {
    // First, extract edit_prediction_provider from features (if present)
    let provider_from_features = obj
        .get_mut("features")
        .and_then(|features| features.as_object_mut())
        .and_then(|features_obj| features_obj.remove("edit_prediction_provider"));

    // Clean up empty features object
    if let Some(features) = obj.get_mut("features") {
        if let Some(features_obj) = features.as_object_mut() {
            if features_obj.is_empty() {
                obj.remove("features");
            }
        }
    }

    // If we extracted a provider from features, insert it into edit_predictions
    // (only if there isn't one already, to preserve explicit user choice)
    if let Some(provider) = provider_from_features {
        let edit_predictions = obj
            .entry("edit_predictions")
            .or_insert_with(|| Value::Object(Default::default()));

        if let Some(edit_predictions_obj) = edit_predictions.as_object_mut() {
            if !edit_predictions_obj.contains_key("provider") {
                let provider = map_xenomorphic_to_copilot(provider);
                edit_predictions_obj.insert("provider".to_string(), provider);
            }
        }
    }

    // Also remap any existing "xenomorphic" provider value to "copilot",
    // since the Xenomorphic variant has been removed from the enum.
    if let Some(edit_predictions) = obj.get_mut("edit_predictions") {
        if let Some(edit_predictions_obj) = edit_predictions.as_object_mut() {
            if let Some(provider) = edit_predictions_obj.get_mut("provider") {
                *provider = map_xenomorphic_to_copilot(provider.clone());
            }
        }
    }

    Ok(())
}

fn map_xenomorphic_to_copilot(value: Value) -> Value {
    if value.as_str() == Some("xenomorphic") {
        Value::String("copilot".to_string())
    } else {
        value
    }
}
