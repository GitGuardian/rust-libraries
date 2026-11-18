use clap::{Parser, Subcommand};
use gg_credential_store::gg_url::DashboardUrl;
use gg_credential_store::{
    CleartextReason, CredentialStore, CredentialStoreError, ReadRequest, Source, StoreOutcome,
    TokenInfo,
};
use serde_json::{Value, json};

const VERSION: &str = match option_env!("GG_TEST_CLIENT_BUILD_TAG") {
    Some(tag) => tag,
    None => env!("CARGO_PKG_VERSION"),
};

/// Runs one operation of the credential store and prints one JSON object.
#[derive(Debug, Parser)]
#[command(name = "gg_credential_store_test_client", version = VERSION)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) operation: Operation,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Operation {
    /// Stores the token read from standard input for an instance.
    Store {
        #[arg(long)]
        instance: DashboardUrl,
        /// Lets the keychain ask to be unlocked.
        #[arg(long)]
        interactive: bool,
    },
    /// Prints the token of an instance and where it was found.
    Read {
        #[arg(long)]
        instance: DashboardUrl,
    },
    /// Removes the token of an instance.
    Delete {
        #[arg(long)]
        instance: DashboardUrl,
    },
}

impl Operation {
    pub(crate) fn run(&self) -> Result<Value, CredentialStoreError> {
        let store = CredentialStore::new()?;
        match self {
            Self::Store { instance, interactive } => {
                let token = std::io::read_to_string(std::io::stdin())
                    .expect("the token is read from standard input");
                let token = token.trim_end();
                let info = TokenInfo {
                    token_type: "personal_access_token".to_owned(),
                    expire_at: None,
                    workspace_id: 1,
                    name: "gg_credential_store_test_client".to_owned(),
                    scopes: Vec::new(),
                };
                let outcome = if *interactive {
                    store.store_interactive(instance, token, &info)?
                } else {
                    store.store(instance, token, &info, None)?
                };
                Ok(match outcome {
                    StoreOutcome::UserOsStore => json!({ "location": "user_os_store" }),
                    StoreOutcome::UserFile { reason, .. } => {
                        let reason = match reason {
                            CleartextReason::Disabled => "disabled",
                            CleartextReason::UnsupportedCaller => "unsupported_caller",
                            CleartextReason::Unavailable => "unavailable",
                            CleartextReason::Locked => "locked",
                            CleartextReason::WriteFailed => "write_failed",
                        };
                        json!({ "location": "user_file", "reason": reason })
                    }
                    StoreOutcome::MachineOsStore => json!({ "location": "machine_os_store" }),
                    StoreOutcome::MachineFile => json!({ "location": "machine_file" }),
                })
            }
            Self::Read { instance } => {
                let credential = store.read(instance, &ReadRequest::default())?;
                let source = match credential.source {
                    Source::MachineOsStore { .. } => "machine_os_store",
                    Source::MachineFile { .. } => "machine_file",
                    Source::Explicit => "explicit",
                    Source::UserOsStore { .. } => "user_os_store",
                    Source::UserFile { .. } => "user_file",
                };
                Ok(json!({ "token": credential.token, "source": source }))
            }
            Self::Delete { instance } => {
                store.delete(instance)?;
                Ok(json!({ "deleted": true }))
            }
        }
    }
}
