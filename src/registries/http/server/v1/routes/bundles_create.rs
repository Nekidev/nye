use async_zip::tokio::read::seek::ZipFileReader;
use axum::body::Bytes;
use futures_util::io::Cursor;
use tokio_util::compat::FuturesAsyncReadCompatExt;

use crate::registries::http::server::state::State;
use crate::registries::http::server::v1::errors::{Error, ResultOrHttpError};
use crate::registries::http::server::v1::extractors::auth::Auth;

pub async fn handle(state: State, auth: Auth, bytes: Bytes) -> Result<(), Error> {
    let file = ZipFileReader::with_tokio(Cursor::new(bytes).compat())
        .await
        .or_http_422()?;

    if file.file().entries().len() > 500 {
        return Err(Error::http_422());
    }

    Ok(())
}
