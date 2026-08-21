use kept_core::schema::export_keyword_registry_schema;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "{}",
        serde_json::to_string_pretty(&export_keyword_registry_schema())?
    );
    Ok(())
}
