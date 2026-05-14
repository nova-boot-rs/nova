use inventory;
use serde_json::{Map, Value, json};

pub struct OpenApiHook {
    pub name: &'static str,
    pub provider: fn() -> Value,
}

inventory::collect!(OpenApiHook);

fn merge_json(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Object(base_map), Value::Object(overlay_map)) => {
            for (key, value) in overlay_map {
                match base_map.get_mut(&key) {
                    Some(existing) => merge_json(existing, value),
                    None => {
                        base_map.insert(key, value);
                    }
                }
            }
        }
        (base_slot, overlay_value) => {
            *base_slot = overlay_value;
        }
    }
}

pub fn build_openapi_document(service_name: &str) -> Value {
    let mut doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("{service_name} API"),
            "version": "0.1.0"
        },
        "paths": Value::Object(Map::new())
    });

    for hook in inventory::iter::<OpenApiHook> {
        let fragment = (hook.provider)();
        merge_json(&mut doc, fragment);
    }

    doc
}
