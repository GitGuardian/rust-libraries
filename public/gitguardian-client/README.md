# gitguardian-client

[![Crates.io](https://img.shields.io/crates/v/gitguardian-client?color=%231B2D55&style=for-the-badge)](https://crates.io/crates/gitguardian-client)
[![docs.rs](https://img.shields.io/docsrs/gitguardian-client?color=%231B2D55&style=for-the-badge)](https://docs.rs/gitguardian-client)
[![License](https://img.shields.io/github/license/GitGuardian/rust-libraries?color=%231B2D55&style=for-the-badge)](https://github.com/GitGuardian/rust-libraries/blob/main/LICENSE)
[![GitHub stars](https://img.shields.io/github/stars/GitGuardian/rust-libraries?color=%231B2D55&style=for-the-badge)](https://github.com/GitGuardian/rust-libraries/stargazers)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/GitGuardian/rust-libraries/release.yml?branch=main&style=for-the-badge)](https://github.com/GitGuardian/rust-libraries/actions)

API client library for the [GitGuardian API](https://api.gitguardian.com/).

The GitGuardian API puts at your fingertips the power to detect more than 600 types of secrets in any text content, as well as other potential security vulnerabilities.

**gitguardian-client** can be used to create integrations to scan various data sources, from your workstation's filesystem to your favorite chat application.

You can check API details [here](https://api.gitguardian.com/docs)
with all the response codes and expected structures on each method.

## MSRV

Rust 1.88+

## Getting started

You can obtain API keys for API usage on your [dashboard](https://dashboard.gitguardian.com/api/v1/auth/user/github_login/authorize?utm_source=github&utm_medium=rs_gitguardian&utm_campaign=rs1).

**cargo**

```bash
cargo add gitguardian-client
```

The HTTP client uses `ureq` by default. To use `reqwest` as an HTTP client, add the crate with

```bash
cargo add gitguardian-client --no-default-features --features reqwest
```

## Usage

Each request is represented by a struct instance that can be sent by the selected client.

### Scanning text content

```rust
let api_key = env::var("GITGUARDIAN_API_KEY")?;
let document = "
    import urllib.request
    url = 'http://jen_barber:correcthorsebatterystaple@cake.gitguardian.com/isreal.json'
    response = urllib.request.urlopen(url)
    consume(response.read())
";

let client = Client::new(ApiConfig::new(&api_key)?);
let request = Scan::new(Document::new(document));

match client.send(&request) {
    Ok(scan_result) => println!("{} policy breaks", scan_result.policy_break_count),
    Err(error) => eprintln!("{error}"),
}
```

### Scanning multiple files

```rust
let api_key = env::var("GITGUARDIAN_API_KEY")?;
let client = Client::new(ApiConfig::new(&api_key)?);

// Create a list of documents for scanning
let request: MultiScan = paths
    .iter()
    .map(|path| {
        let content = fs::read(path).unwrap_or_default();
        Document::new(String::from_utf8_lossy(&content))
            .with_filename(path.file_name().unwrap_or_default().to_string_lossy())
    })
    .collect();

let scan = client.send(&request)?;
```

### Paginating results

Paginated routes are iterators that follow the cursor until the last page.

```rust
let api_key = env::var("GITGUARDIAN_API_KEY")?;
let client = Client::new(ApiConfig::new(&api_key)?);

for page in client.paginate(ListTeams::default()) {
    for team in page?.items {
        println!("{}", team.name);
    }
}
```

### Using the async client

With the `reqwest` feature, `send` is awaited.

```rust
let api_key = env::var("GITGUARDIAN_API_KEY")?;
let client = Client::new(ApiConfig::new(&api_key)?);

let quotas = client.send(&RetrieveQuotas).await?;
println!("{} API calls remaining", quotas.content.remaining);
```

## Architecture

This project exposes two crates in order to decouple the API representation from the HTTP client. `gitguardian-api` provides strong typing around GitGuardian’s API, as well as a set of models that maps to GitGuardian’s types. `gitguardian-client` exposes client wrappers for `ureq` agents and `reqwest` clients. The `reqwest` client is feature-gated. The `reqwest` client is also reasonably generic so that it works with reqwest middleware such as `http-cache`.
