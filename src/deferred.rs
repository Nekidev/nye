//! Deferred execution of code.

use std::marker::PhantomData;

pub struct Deferred<F, R>(Option<F>, PhantomData<R>)
where
    F: Deferrable<R>,
    R: Send + Sync + 'static;

impl<F, R> Deferred<F, R>
where
    F: Deferrable<R>,
    R: Send + Sync + 'static,
{
    fn cancel(&mut self) {
        self.0.take();
    }
}

impl<F, R> Drop for Deferred<F, R>
where
    F: Deferrable<R>,
    R: Send + Sync + 'static,
{
    fn drop(&mut self) {
        if let Some(task) = self.0.take() {
            tokio::spawn(task);
        }
    }
}

/// Defers the execution of the future until the guard is dropped.
///
/// For example:
/// ```
/// let _1 = deferred::defer(async {
///     println!("hi!");
/// });
/// ```
pub fn defer<F, R>(task: F) -> Deferred<F, R>
where
    F: Deferrable<R>,
    R: Send + Sync,
{
    Deferred(Some(task), PhantomData)
}

pub trait Deferrable<R>: Future<Output = R> + Send + Sync + 'static
where
    R: Send + Sync + 'static,
{
}
impl<F, R> Deferrable<R> for F
where
    F: Future<Output = R> + Send + Sync + 'static,
    R: Send + Sync + 'static,
{
}

pub trait ElidedDeferrable: Send + Sync + 'static {
    fn cancel(&mut self);
}

impl<F, R> ElidedDeferrable for Deferred<F, R>
where
    F: Future<Output = R> + Send + Sync + 'static,
    R: Send + Sync + 'static,
{
    fn cancel(&mut self) {
        Deferred::cancel(self);
    }
}

/// Defer multiple futures with a single guard.
///
/// For example,
/// ```
/// let mut deferred = DeferredGroup::default();
/// deferred.defer(async { println!("Hi") });
/// ```
#[derive(Default)]
pub struct DeferredGroup(Vec<Box<dyn ElidedDeferrable>>);

impl DeferredGroup {
    /// Add a deferred future to the guard.
    ///
    /// For example,
    /// ```
    /// let mut deferred = DeferredGroup::default();
    /// deferred.defer(async { println!("Hi") });
    /// ```
    pub fn defer<F, R>(&mut self, task: F)
    where
        F: Deferrable<R>,
        R: Send + Sync + 'static,
    {
        self.0.push(Box::new(defer(task)));
    }

    pub fn cancel(mut self) {
        for deferred in &mut self.0 {
            deferred.cancel();
        }
    }
}
