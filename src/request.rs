use url::Url;
use gemini_fetch::Page;
use anyhow::Result;
use tokio::runtime::Runtime;

async fn get_gemini_page(address: &Url) -> Result<String> {
    match Page::fetch(address, None).await {
        Ok(page) => {
            // Handle the fetched Gemini page
            println!("URL: {}", page.url);
            println!("Status: {:?}", page.header.status);
            println!("Meta: {}", page.header.meta);
            if let Some(body) = page.body {
                Ok(body)
            } else {
                Ok("No body found in the Gemini page".to_string())
            }
        }
        Err(err) => {
            // Handle errors
            eprintln!("Error: {}", err);
            Ok("Error fetching Gemini page".to_string())
        }
    }
}

fn get_gemini_page_blocking(address: &Url) -> Result<String> {
    Runtime::new().unwrap().block_on(get_gemini_page(address))
}

fn handle_address(base_path: &str, address: &str) -> Result<String> {
    if address.starts_with("gemini://") || address.starts_with("http://") || address.starts_with("https://") {
        return Ok(address.to_string());
    } else {
        // relative path
        let absolute_path = resolve_url_path(base_path, address);
        Ok(absolute_path)
    }
}

fn resolve_url_path(base_path: &str, relative_path: &str) -> String {
    let base_url = Url::parse(base_path).expect("Failed to parse base URL");
    let resolved_url = base_url.join(relative_path).expect("Failed to resolve URL");

    resolved_url.into_string()
}

pub fn fetch_page(address: &str, base_path: &str) -> String {
    let address = handle_address(base_path, address).unwrap();
    let gemini_url = Url::parse(&address).expect("Invalid URL");

    let gemini_body = get_gemini_page_blocking(&gemini_url).expect("Error fetching Gemini page");
    gemini_body
}
