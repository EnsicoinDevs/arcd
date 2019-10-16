use service_book::ServiceList;

// TODO: Get better
pub async fn get_peers(api_path: &str) -> Option<ServiceList> {
    match reqwest::get(&format!("{}/discover/XEC", api_path)).await {
        Err(_) => None,
        Ok(mut d) => None,
    }
}

#[derive(Debug)]
pub enum DiscoverError {
    RegisterFailed,
    RegisterRevoked,
    RequestError(reqwest::Error),
}
