use reqwest::r#async::Client as AsyncClient;
use service_book::ServiceList;

pub fn get_peers(api_path: &str) -> Option<ServiceList> {
    match reqwest::get(&format!("{}/discover/XEC", api_path)) {
        Err(_) => None,
        Ok(mut d) => match d.json() {
            Err(_) => None,
            Ok(l) => Some(l),
        },
    }
}

#[derive(Debug)]
pub enum DiscoverError {
    RegisterFailed,
    RegisterRevoked,
    RequestError(reqwest::Error),
}
