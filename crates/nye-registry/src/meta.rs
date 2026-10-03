//! Registry metadata.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryMeta {
    /// The registry's display name.
    pub name: String,

    /// The registry's suggested alias to be stored as.
    ///
    /// When doubting what to set this value to, set it to an empty string.
    pub suggested_alias: String,

    /// The signin Duckity policy ID, if set up.
    pub duckity_signin_policy_id: Option<String>,
    /// The signup Duckity policy ID, if set up.
    pub duckity_signup_policy_id: Option<String>,

    /// Whether account creations are enabled.
    pub is_signup_enabled: bool,
    /// Whether logging in is enabled.
    pub is_signin_enabled: bool,
}
