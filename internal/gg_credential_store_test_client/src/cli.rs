use clap::{Parser, Subcommand};
use gg_credential_store::gg_url::DashboardUrl;
use gg_credential_store::{
    CredentialStore, CredentialStoreError, DeleteOutcome, OsStoreErrorReason, ReadRequest, Source,
    StoreOutcome, TokenInfo, TokenType,
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
        /// Lets the OS store ask the user to unlock it.
        #[arg(long)]
        interactive: bool,
        /// The machine id recorded with a machine token.
        #[arg(long)]
        machine_id: Option<String>,
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
    /// Prints the instance of the machine token.
    ManagedInstance,
}

impl Operation {
    pub(crate) fn run(&self) -> Result<Value, CredentialStoreError> {
        let store = CredentialStore::new()?;
        match self {
            Self::Store { instance, interactive, machine_id } => {
                let token = std::io::read_to_string(std::io::stdin())
                    .expect("the token is read from standard input");
                let token = token.trim_end();
                let info = TokenInfo {
                    token_type: TokenType::PersonalAccessToken,
                    expire_at: None,
                    workspace_id: 1,
                    name: "gg_credential_store_test_client".to_owned(),
                    scopes: Vec::new(),
                };
                let outcome = if *interactive {
                    store.store_interactive(instance, token, &info)?
                } else {
                    store.store(instance, token, &info, machine_id.as_deref())?
                };
                Ok(match outcome {
                    StoreOutcome::UserOsStore => json!({ "location": "user_os_store" }),
                    StoreOutcome::UserFile { reason, .. } => {
                        with_reason("location", "user_file", &reason)
                    }
                    StoreOutcome::MachineOsStore => json!({ "location": "machine_os_store" }),
                    StoreOutcome::MachineFile { reason, .. } => {
                        with_reason("location", "machine_file", &reason)
                    }
                })
            }
            Self::Read { instance } => {
                let credential = store.read(instance, &ReadRequest::default())?;
                let (source, machine_id) = match credential.source {
                    Source::MachineOsStore { machine_id } => ("machine_os_store", Some(machine_id)),
                    Source::MachineFile { machine_id } => ("machine_file", Some(machine_id)),
                    Source::Explicit => ("explicit", None),
                    Source::UserOsStore { .. } => ("user_os_store", None),
                    Source::UserFile { .. } => ("user_file", None),
                };
                Ok(json!({ "token": credential.token, "source": source, "machine_id": machine_id }))
            }
            Self::Delete { instance } => Ok(match store.delete(instance)? {
                DeleteOutcome::Removed => json!({ "outcome": "removed" }),
                DeleteOutcome::NotStored => json!({ "outcome": "not_stored" }),
                DeleteOutcome::KeptInOsStore(reason) => {
                    with_reason("outcome", "kept_in_os_store", &reason)
                }
            }),
            Self::ManagedInstance => {
                let instance = store.managed_instance()?.map(|instance| instance.to_string());
                Ok(json!({ "instance": instance }))
            }
        }
    }
}

fn with_reason(key: &str, value: &str, reason: &OsStoreErrorReason) -> Value {
    let (name, detail) = match reason {
        OsStoreErrorReason::Disabled => ("disabled", None),
        OsStoreErrorReason::UnsupportedCaller => ("unsupported_caller", None),
        OsStoreErrorReason::Unavailable(detail) => ("unavailable", Some(detail)),
        OsStoreErrorReason::Locked => ("locked", None),
        OsStoreErrorReason::Refused(detail) => ("refused", Some(detail)),
    };
    json!({ key: value, "reason": name, "detail": detail })
}
