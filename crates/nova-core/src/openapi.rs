use crate::runtime::NovaRoute;
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

fn default_paths_from_routes() -> Map<String, Value> {
    let mut paths = Map::new();

    for route in inventory::iter::<NovaRoute> {
        let method = route.method.to_lowercase();
        let operation = json!({
            "operationId": format!("{}_{}", method, route.path.replace('/', "_").trim_matches('_')),
            "responses": {
                "200": {
                    "description": "Successful response"
                }
            }
        });

        let entry = paths
            .entry(route.path.to_string())
            .or_insert_with(|| Value::Object(Map::new()));

        if let Value::Object(path_item) = entry {
            path_item.insert(method, operation);
        }
    }

    paths
}

pub fn build_openapi_document(service_name: &str) -> Value {
    let mut doc = json!({
        "openapi": "3.1.0",
        "info": {
            "title": format!("{service_name} API"),
            "version": "0.1.0"
        },
        "paths": Value::Object(default_paths_from_routes())
    });

    for hook in inventory::iter::<OpenApiHook> {
        let fragment = (hook.provider)();
        merge_json(&mut doc, fragment);
    }

    doc
}
