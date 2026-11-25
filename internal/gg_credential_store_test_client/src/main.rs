mod cli;

use clap::Parser;
use gg_credential_store::Cause;
use serde_json::json;

fn main() {
    let (output, code) = match cli::Cli::parse().operation.run() {
        Ok(result) => (json!({ "ok": true, "result": result }), 0),
        Err(err) => {
            let (kind, code) = match err.cause() {
                Cause::OsStoreDenied => ("os_store_denied", 2),
                Cause::OsStoreItemMissing => ("os_store_item_missing", 3),
                Cause::OsStoreLocked(_) => ("os_store_locked", 4),
                Cause::OsStoreUnavailable(_) => ("os_store_unavailable", 5),
                Cause::NoToken => ("no_token", 6),
                Cause::MachineStoreDenied => ("machine_store_denied", 7),
                Cause::MissingMachineId => ("missing_machine_id", 8),
                Cause::MachineTokenSkipped(_) => ("machine_token_skipped", 9),
                _ => ("other", 1),
            };
            (json!({ "ok": false, "error": { "kind": kind, "message": err.to_string() } }), code)
        }
    };
    println!("{output}");
    std::process::exit(code);
}
