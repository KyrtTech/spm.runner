use crate::cli;
use crate::config::RegistryConfig;

pub fn run(
    name: &str,
    category: cli::SearchCategory,
    config: &RegistryConfig,
) -> Result<(), url::ParseError> {
    let mut search_url = config.get_url().join("search")?;

    let url_with_query = search_url
        .query_pairs_mut()
        .append_pair("name", name)
        .append_pair("type", &category.to_string())
        .finish();

    println!(
        "Doing search for: {}, {}. Using url {}",
        name, category, url_with_query
    );

    Ok(())
}