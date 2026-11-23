/// Where a token is stored.
///
/// A machine token carries the machine id recorded when it was stored.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Location {
    /// A user token in the user's credential store of the OS.
    UserOsStore,
    /// A user token in clear text in `auth_config.yaml`.
    UserFile,
    /// A machine token in the machine-wide credential store of the OS.
    MachineOsStore {
        /// The machine id recorded with the token.
        machine_id: String,
    },
    /// A machine token in `machine_credential.yaml`.
    MachineFile {
        /// The machine id recorded with the token.
        machine_id: String,
    },
}
