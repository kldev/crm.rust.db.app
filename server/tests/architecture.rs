//! ADR-0013: the API calls `app` procedures, functions and views and nothing else; no ORM or query mapper
//! (ArchitectureTest.kt, verifyNoOrm).

use std::{fs, path::Path};

fn sources(dir: &Path, out: &mut Vec<(String, String)>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push((
                path.display().to_string(),
                fs::read_to_string(&path).unwrap(),
            ));
        }
    }
}

/// String literals of the API code; `migrate.rs` and `seed.rs` (CRM_MODE=init/migrate/seed, not the API) are the
/// exceptions.
fn literals() -> Vec<(String, String)> {
    let mut files = Vec::new();
    sources(Path::new("src"), &mut files);
    let literal = regex_lite::Regex::new(r#"(?s)"(?:[^"\\]|\\.)*""#).unwrap();
    let continuation = regex_lite::Regex::new(r"\\\n\s*").unwrap();
    files
        .into_iter()
        .filter(|(name, _)| !name.ends_with("migrate.rs") && !name.ends_with("seed.rs"))
        .flat_map(|(name, text)| {
            literal
                .find_iter(&text)
                .map(|m| {
                    (
                        name.clone(),
                        continuation.replace_all(m.as_str(), "").into_owned(),
                    )
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

#[test]
fn every_database_statement_is_a_call_or_a_read_of_schema_app() {
    let sql_word = regex_lite::Regex::new(
        r"\b(SELECT|INSERT|UPDATE|DELETE|MERGE|TRUNCATE|CREATE|ALTER|DROP|CALL|EXEC|COPY)\b",
    )
    .unwrap();
    let allowed = regex_lite::Regex::new(
        r#"^"(CALL app\.[a-z_]+\(|SELECT [^;]+? FROM app\.[a-z_]+(\(|"| ))[^;]*"$"#,
    )
    .unwrap();
    let statements: Vec<_> = literals()
        .into_iter()
        .filter(|(_, l)| sql_word.is_match(l))
        .collect();
    assert!(
        statements.len() >= 10,
        "expected the statements to be found, got {}",
        statements.len()
    );
    let wrong: Vec<_> = statements
        .iter()
        .filter(|(_, l)| !allowed.is_match(l))
        .collect();
    assert!(
        wrong.is_empty(),
        "statements outside schema app: {wrong:#?}"
    );
}

#[test]
fn no_orm_or_query_mapper_in_the_dependencies() {
    let lock = fs::read_to_string("Cargo.lock").unwrap();
    let forbidden = [
        "diesel",
        "sea-orm",
        "sqlx",
        "cornucopia",
        "clorinde",
        "postgres-from-row",
        "tokio-pg-mapper",
    ];
    let found: Vec<_> = forbidden
        .iter()
        .filter(|name| lock.contains(&format!("name = \"{name}\"")))
        .collect();
    assert!(found.is_empty(), "forbidden crates: {found:?}");
}
