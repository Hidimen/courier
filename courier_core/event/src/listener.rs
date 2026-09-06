use std::{any::TypeId, marker::PhantomData, pin::Pin};

use crate::{Event, error::ListenerError};

type AsyncResult<'a> =
  Pin<Box<dyn Future<Output = Result<(), ListenerError>> + Send + Sync + 'a>>;

type SyncHandler =
  dyn Fn(&dyn Event) -> Result<(), ListenerError> + Send + Sync;

/// Object-safe erasure of an async listener. The returned future may borrow
/// both the listener (`&'a self`) and the event (`&'a dyn Event`).
pub(crate) trait AsyncHandler: Send + Sync {
  fn handle<'a>(&'a self, event: &'a (dyn Event + 'static)) -> AsyncResult<'a>;
}

pub trait Listener<E: Event>: Send + Sync + 'static {
  fn handle(&self, event: &E) -> Result<(), ListenerError>;
}

pub trait AsyncListener<E: Event>: Send + Sync + 'static {
  fn handle(
    &self, event: &E,
  ) -> impl Future<Output = Result<(), ListenerError>> + Send + Sync;
}

impl<F, E> Listener<E> for F
where
  E: Event,
  F: Fn(&E) -> Result<(), ListenerError> + Send + Sync + 'static,
{
  fn handle(&self, event: &E) -> Result<(), ListenerError> {
    (self)(event)
  }
}

impl<F, E, Fut> AsyncListener<E> for F
where
  E: Event,
  F: Fn(&E) -> Fut + Send + Sync + 'static,
  Fut: Future<Output = Result<(), ListenerError>> + Send + Sync + 'static,
{
  async fn handle(&self, event: &E) -> Result<(), ListenerError> {
    (self)(event).await
  }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct ListenerId {
  pub type_id: TypeId,
  pub id: usize,
}

pub struct ListenerWrapper {
  pub(crate) id: ListenerId,
  pub(crate) handler: Box<SyncHandler>,
}

impl ListenerWrapper {
  pub fn new<L: Listener<E>, E: Event>(listener: L, id: ListenerId) -> Self {
    Self {
      id,
      handler: Box::new(
        move |event: &dyn Event| -> Result<(), ListenerError> {
          if let Some(evt) = event.as_any().downcast_ref::<E>() {
            listener.handle(evt)
          } else {
            debug_assert!(false, "Wrong event type. It's unreachable!");
            Ok(())
          }
        },
      ),
    }
  }

  pub fn execute(
    &self, event: &(dyn Event + 'static),
  ) -> Result<(), ListenerError> {
    (self.handler)(event)
  }
}

pub struct AsyncListenerWrapper {
  pub(crate) id: ListenerId,
  pub(crate) handler: Box<dyn AsyncHandler>,
}

impl AsyncListenerWrapper {
  pub fn new<L: AsyncListener<E>, E: Event>(
    listener: L, id: ListenerId,
  ) -> Self {
    Self {
      id,
      handler: Box::new(AsyncListenerAdapter {
        listener,
        _marker: PhantomData,
      }),
    }
  }

  pub async fn execute(
    &self, event: &(dyn Event + 'static),
  ) -> Result<(), ListenerError> {
    self.handler.handle(event).await
  }
}

struct AsyncListenerAdapter<L, E> {
  listener: L,
  _marker: PhantomData<E>,
}

impl<L, E> AsyncHandler for AsyncListenerAdapter<L, E>
where
  L: AsyncListener<E>,
  E: Event,
{
  fn handle<'a>(&'a self, event: &'a (dyn Event + 'static)) -> AsyncResult<'a> {
    if let Some(evt) = event.as_any().downcast_ref::<E>() {
      Box::pin(async move { self.listener.handle(evt).await })
    } else {
      debug_assert!(false, "Wrong event type. It's unreachable!");
      Box::pin(async move { Ok(()) })
    }
  }
}
