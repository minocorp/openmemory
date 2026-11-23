use std::fs;
use std::path::Path;

use crate::api::openapi;

fn rust_sources(directory: &Path, found: &mut Vec<String>) {
    for entry in fs::read_dir(directory).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            rust_sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs")
            && path.file_name().is_some_and(|name| name != "tests.rs")
            && !path.components().any(|part| part.as_os_str() == "tests")
        {
            found.push(fs::read_to_string(&path).unwrap());
        }
    }
}

fn api_literals(source: &str) -> Vec<String> {
    source
        .split("\"/api/")
        .skip(1)
        .filter_map(|rest| rest.split_once('"'))
        .map(|(path, _)| format!("/api/{path}"))
        .filter(|route| !route.contains("{}") && !route.ends_with('/'))
        .collect()
}

#[test]
fn every_api_route_literal_in_the_source_is_documented() {
    let mut sources = Vec::new();
    rust_sources(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("src"),
        &mut sources,
    );
    let document = openapi();
    let documented: Vec<&String> = document.paths.paths.keys().collect();
    let missing: Vec<String> = sources
        .iter()
        .flat_map(|source| api_literals(source))
        .filter(|route| !documented.contains(&route))
        .collect();
    assert_eq!(missing, Vec::<String>::new());
}

#[test]
fn the_document_has_a_session_security_scheme_and_error_schema() {
    let document = serde_json::to_value(openapi()).unwrap();
    assert_eq!(
        document["components"]["securitySchemes"]["session"]["in"],
        "cookie"
    );
    assert!(document["components"]["schemas"]["ErrorBody"].is_object());
}
