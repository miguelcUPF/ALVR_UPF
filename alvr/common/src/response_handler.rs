use reqwest::blocking::Response;

pub fn handle_ap_response(response: Response) -> Result<String, String> {
    if response.status().is_success() {
        match response.text() {
            Ok(body) => Ok(body),
            Err(err) => Err(format!("Failed to read response body: {}", err.to_string())),
        }
    } else {
        Err(format!(
            "Failed to get a successful response: {}",
            response.status()
        ))
    }
}
