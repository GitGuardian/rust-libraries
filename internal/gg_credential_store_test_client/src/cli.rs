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
                    token_type: "personal_access_token".to_owned(),
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
                    StoreOutcome::UserFile { reason, .. } => cleartext("user_file", &reason),
                    StoreOutcome::MachineOsStore => json!({ "location": "machine_os_store" }),
                    StoreOutcome::MachineFile { reason, .. } => cleartext("machine_file", &reason),
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
            Self::Delete { instance } => {
                store.delete(instance)?;
                Ok(json!({ "deleted": true }))
            }
            Self::ManagedInstance => {
                let instance = store.managed_instance()?.map(|instance| instance.to_string());
                Ok(json!({ "instance": instance }))
            }
        }
    }
}

fn cleartext(location: &str, reason: &CleartextReason) -> Value {
    let (name, detail) = match reason {
        CleartextReason::Disabled => ("disabled", None),
        CleartextReason::UnsupportedCaller => ("unsupported_caller", None),
        CleartextReason::Unavailable(detail) => ("unavailable", Some(detail)),
        CleartextReason::Locked => ("locked", None),
        CleartextReason::WriteFailed(detail) => ("write_failed", Some(detail)),
    };
    json!({ "location": location, "reason": name, "detail": detail })
}
