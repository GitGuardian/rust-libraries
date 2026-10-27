use serde_json::{Value, json};

pub fn patch(spec: &mut Value) {
    patch_security_schemes(spec);
    patch_nullable_enums(spec);
    patch_project_extensions(spec);
    patch_honeytoken_context_filename(spec);
    patch_scan_create_incidents_result(spec);
    patch_validity_examples(spec);
    patch_metadata_path(spec);
}

fn patch_validity_examples(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if map.get("validity").and_then(Value::as_str) == Some("cannot_check") {
                map.insert("validity".to_owned(), json!("no_checker"));
            }
            for value in map.values_mut() {
                patch_validity_examples(value);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(patch_validity_examples),
        _ => {}
    }
}

fn patch_project_extensions(spec: &mut Value) {
    let Some(schema) = spec.pointer_mut(
        "/paths/~1v1~1honeytokens~1with-context/post/requestBody/content/application~1json/schema/properties/project_extensions",
    ) else {
        return;
    };
    *schema = json!({
        "type": "string",
        "description": "Comma separated file extensions that can be used for the context.",
        "example": ".c,.h",
    });
}

fn patch_honeytoken_context_filename(spec: &mut Value) {
    let Some(properties) = spec
        .pointer_mut(
            "/paths/~1v1~1honeytokens~1with-context/post/responses/200/content/application~1json/schema/properties",
        )
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    if let Some(filepath) = properties.remove("filepath") {
        properties.insert("filename".to_owned(), filepath);
    }
}

fn patch_scan_create_incidents_result(spec: &mut Value) {
    let Some(schema) = spec.pointer_mut(
        "/paths/~1v1~1scan~1create-incidents/post/responses/200/content/application~1json/schema",
    ) else {
        return;
    };
    if schema.get("type").and_then(Value::as_str) != Some("object") {
        return;
    }
    *schema = json!({
        "type": "array",
        "minItems": 1,
        "items": schema.take(),
    });
}

fn patch_security_schemes(spec: &mut Value) {
    let Some(schemes) =
        spec.pointer_mut("/components/securitySchemes").and_then(Value::as_object_mut)
    else {
        return;
    };

    for scheme in schemes.values_mut() {
        if scheme.get("type").and_then(Value::as_str) != Some("http") {
            continue;
        }
        let description = scheme.get("description").cloned();
        let mut replacement = json!({
            "type": "apiKey",
            "in": "header",
            "name": "Authorization",
        });
        if let Some(description) = description
            && let Some(fields) = replacement.as_object_mut()
        {
            fields.insert("description".into(), description);
        }
        *scheme = replacement;
    }
}

fn patch_nullable_enums(node: &mut Value) {
    match node {
        Value::Object(map) => {
            if map.get("nullable").and_then(Value::as_bool) == Some(true)
                && let Some(values) = map.get_mut("enum").and_then(Value::as_array_mut)
                && !values.iter().any(Value::is_null)
            {
                values.push(Value::Null);
            }
            for value in map.values_mut() {
                patch_nullable_enums(value);
            }
        }
        Value::Array(values) => values.iter_mut().for_each(patch_nullable_enums),
        _ => {}
    }
}

fn patch_metadata_path(spec: &mut Value) {
    let Some(paths) = spec.get_mut("paths").and_then(Value::as_object_mut) else {
        return;
    };
    paths.insert(
        "/v1/metadata".to_owned(),
        json!({
            "get": {
                "summary": "Get public Metadata",
                "responses": {
                    "200": {
                        "description": "Get public Metadata",
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "required": [
                                        "version",
                                        "preferences",
                                        "secret_scan_preferences",
                                        "remediation_messages"
                                    ],
                                    "properties": {
                                        "version": {"type": "string", "example": "1.0.0"},
                                        "preferences": {
                                            "type": "object",
                                            "additionalProperties": true,
                                            "example": {"general__maximum_payload_size": 26214400}
                                        },
                                        "secret_scan_preferences": {
                                            "type": "object",
                                            "required": ["maximum_documents_per_scan", "maximum_document_size"],
                                            "properties": {
                                                "maximum_documents_per_scan": {"type": "integer", "example": 20},
                                                "maximum_document_size": {"type": "integer", "example": 1048576}
                                            }
                                        },
                                        "remediation_messages": {
                                            "type": "object",
                                            "additionalProperties": {"type": "string"},
                                            "example": {
                                                "pre_commit": "Docs: https://docs.gitguardian.com",
                                                "pre_push": "Docs: https://docs.gitguardian.com",
                                                "pre_receive": "Docs: https://docs.gitguardian.com"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }),
    );
}
