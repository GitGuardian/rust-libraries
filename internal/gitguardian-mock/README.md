# gitguardian-mock

This crate starts a local mock of the GitGuardian API for the end to end tests of `gitguardian-client`.

The mock runs [Prism](https://github.com/stoplightio/prism) through `npx` against the public OpenAPI specification. Node.js must be installed. The crate supports Unix targets only.

The crate is internal. It is not published and it is not released.
