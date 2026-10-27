use gitguardian_client::reqwest::Client;
use gitguardian_mock::{MockServer, prefer};

pub fn http_preferring(status: u16) -> ::reqwest::Client {
    let mut headers = ::reqwest::header::HeaderMap::new();
    headers.insert(
        prefer::HEADER,
        ::reqwest::header::HeaderValue::from_str(&prefer::code(status)).unwrap(),
    );
    ::reqwest::Client::builder().default_headers(headers).build().unwrap()
}

pub fn client(server: &MockServer) -> Client {
    Client::new(super::config(server))
}

pub fn client_preferring(server: &MockServer, status: u16) -> Client {
    Client::with_http_client(super::config(server), http_preferring(status))
}
