use std::time::Duration;

use anyhow::Context;

use crate::registries::http::server::database::Token;
use crate::registries::http::server::state::RegistryState;

/// Loops, cleaning up expired tokens from the database every minute.
///
/// This function does not return unless an error occurs.
///
/// Arguments:
/// * `state` - The registry's state.
///
/// Returns:
/// * `Err(Error)` - An error, if any occurred.
pub async fn run(state: RegistryState) -> anyhow::Result<()> {
    let mut db = state
        .db
        .connection()
        .await
        .context("Could not connect to database.")?;

    loop {
        tokio::time::sleep(Duration::from_mins(1)).await;

        Token::filter(Token::fields().expires_at().lt(crate::time::utc_now_ms()))
            .delete()
            .exec(&mut db)
            .await
            .context("Could not delete all expired tokens.")?;
    }
}
