//! [`Loadable`] trait definition and implementations.
//!
//! The [`Loadable`] trait is implemented by types that load a file entry's contents to write them
//! to a package file. There are already a few useful implementations of the trait for generic
//! types, and you'll rarely want to implement the trait on your own.
//!
//! To create a custom loader, you most likely want to create a type that implements [`AsyncRead`]
//! instead.

use std::marker::PhantomData;
use std::pin::Pin;

use tokio::io::AsyncRead;

pub(crate) struct LoadableWrapper<T, Kind> {
    pub inner: T,
    pub _phantom: PhantomData<Kind>,
}

/// Phantom type for [`AsyncRead`]-implementing types.
pub struct PhantomDirect;
/// Phantom type for futures that return [`AsyncRead`]-implementing types.
pub struct PhantomFuture;
/// Phantom type for futures that return a result holding the [`AsyncRead`]-implementing type.
pub struct PhantomFutureResult;
/// Phantom type for functions that return an [`AsyncRead`]-implementing type.
pub struct PhantomFnOnce;
/// Phantom type for functions that return a result holding the [`AsyncRead`]-implementing type.
pub struct PhantomFnOnceResult;
/// Phantom type for aync functions that return an [`AsyncRead`]-implementing type.
pub struct PhantomAsyncFnOnce;
/// Phantom type for async functions that return a result holding the [`AsyncRead`]-implementing
/// type.
pub struct PhantomAsyncFnOnceResult;

/// Trait that returns an [`AsyncRead`] type for writing into a package file.
///
/// Implementing this trait requires a phantom type to prevent collisions. This module exposes
/// `Phantom*` types for the provided implementations, create yours following the same pattern.
/// 
/// For example,
/// 
/// ```
/// struct PhantomCustomType;
/// 
/// impl Loadable<PhantomCustomType> for MyLoader {
///     async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
///         todo!()
///     }
/// }
/// ```
pub trait Loadable<Kind> {
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>>;
}

impl<T> Loadable<PhantomDirect> for T
where
    T: AsyncRead + Unpin + 'static,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
        Ok(Box::new(self))
    }
}

impl<F, P> Loadable<PhantomFuture> for F
where
    F: Future<Output = P>,
    P: AsyncRead + Unpin + 'static,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
        Ok(Box::new(self.await))
    }
}

impl<F, P, E> Loadable<PhantomFutureResult> for F
where
    F: Future<Output = Result<P, E>>,
    P: AsyncRead +Unpin+ 'static,
    E: Into<anyhow::Error>,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
        match self.await {
            Ok(value) => Ok(Box::new(value)),
            Err(error) => Err(error.into()),
        }
    }
}

impl<F, P> Loadable<PhantomFnOnce> for F
where
    F: FnOnce() -> P,
    P: AsyncRead + Unpin + 'static,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
        Ok(Box::new(self()))
    }
}

impl<F, P, E> Loadable<PhantomFnOnceResult> for F
where
    F: FnOnce() -> Result<P, E>,
    P: AsyncRead + Unpin + 'static,
    E: Into<anyhow::Error>,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin+ 'static>> {
        match self() {
            Ok(value) => Ok(Box::new(value)),
            Err(error) => Err(error.into()),
        }
    }
}

impl<F, P> Loadable<PhantomAsyncFnOnce> for F
where
    F: AsyncFnOnce() -> P,
    P: AsyncRead + Unpin + 'static,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
        Ok(Box::new(self().await))
    }
}

impl<F, P, E> Loadable<PhantomAsyncFnOnceResult> for F
where
    F: AsyncFnOnce() -> Result<P, E>,
    P: AsyncRead + Unpin + 'static,
    E: Into<anyhow::Error>,
{
    async fn load(self) -> anyhow::Result<Box<dyn AsyncRead + Unpin + 'static>> {
        match self().await {
            Ok(value) => Ok(Box::new(value)),
            Err(error) => Err(error.into()),
        }
    }
}

pub(crate) trait LoadableWithoutKind {
    fn load(self: Box<Self>) -> Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn AsyncRead + Unpin>>>>>;
}

impl<T, Kind> LoadableWithoutKind for LoadableWrapper<T, Kind>
where
    T: Loadable<Kind> + 'static,
    Kind: 'static,
{
    fn load(self: Box<Self>) -> Pin<Box<dyn Future<Output = anyhow::Result<Box<dyn AsyncRead + Unpin>>>>> {
        Box::pin(async move { self.inner.load().await })
    }
}
