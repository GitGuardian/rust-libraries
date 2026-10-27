# gitguardian-api

This crate defines the requests and models of the [GitGuardian API](https://api.gitguardian.com/docs).

Each request is a struct. It knows its method, its path and its response type. The crate does not send requests. [`gitguardian-client`](https://crates.io/crates/gitguardian-client) sends them with `ureq` or `reqwest`.

Most users add `gitguardian-client` and use the types it reexports.

```bash
cargo add gitguardian-client
```

The versions of `gitguardian-api` and `gitguardian-client` move together. `gitguardian-client` requires the exact version of `gitguardian-api` it was released with.

## MSRV

Rust 1.88+
