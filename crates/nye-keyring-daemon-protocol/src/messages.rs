use bitcode::{Decode, Encode};

#[derive(Encode, Decode)]
pub enum ClientMessage {
    /// Get the access token for a registry.
    GetAuthForRegistry(ClientGetAuthForRegistry),

    /// Sets the access and refresh tokens for a registry.
    SetAuthForRegistry(ClientSetAuthForRegistry),
}

#[derive(Encode, Decode)]
pub struct ClientGetAuthForRegistry {
    pub registry_url: String,
}

impl From<ClientGetAuthForRegistry> for ClientMessage {
    fn from(value: ClientGetAuthForRegistry) -> Self {
        ClientMessage::GetAuthForRegistry(value)
    }
}

#[derive(Encode, Decode)]
pub struct ClientSetAuthForRegistry {
    pub registry_url: String,
    pub access_token: String,
    pub access_token_expires_in: u64,
    pub refresh_token: String,
    pub refresh_token_expires_in: u64,
}

impl From<ClientSetAuthForRegistry> for ClientMessage {
    fn from(value: ClientSetAuthForRegistry) -> Self {
        ClientMessage::SetAuthForRegistry(value)
    }
}

#[derive(Encode, Decode)]
pub enum ServerMessage {
    /// An error occurred inside the keyring daemon.
    Error(String),

    /// An empty response.
    Nothing,

    /// Returns the access token for a registry.
    RegistryAuth(ServerRegistryAuth),
}

#[derive(Encode, Decode)]
pub struct ServerRegistryAuth {
    pub access_token: String,
}

impl From<ServerRegistryAuth> for ServerMessage {
    fn from(value: ServerRegistryAuth) -> Self {
        ServerMessage::RegistryAuth(value)
    }
}
