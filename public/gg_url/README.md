# gg_url

This crate parses and validates URLs for GitGuardian products.

`HttpUrl` holds an HTTP or HTTPS URL. `ApiUrl` and `DashboardUrl` wrap it for the GitGuardian API and dashboard. `GenericUri` holds a URI of any scheme. `RequestUrl` resolves a request target against a base URL. `Host` and `Query` expose the host and the query of a URL.

```bash
cargo add gg_url
```
