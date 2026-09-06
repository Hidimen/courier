use std::{
  any::TypeId,
  sync::{
    Arc, OnceLock,
    atomic::{AtomicUsize, Ordering},
  },
};

use dashmap::DashMap;
use tokio::{select, sync::mpsc::Sender};
use tokio_util::sync::CancellationToken;

use crate::{
  Event,
  listener::{
    AsyncListener, AsyncListenerWrapper, Listener, ListenerId, ListenerWrapper,
  },
};

static DISPATCHER: OnceLock<Arc<Dispatcher>> = OnceLock::new();

pub struct Dispatcher {
  sender: Sender<Box<dyn Event>>,
  listeners: Arc<DashMap<TypeId, Vec<ListenerWrapper>>>,
  async_listeners: Arc<DashMap<TypeId, Vec<AsyncListenerWrapper>>>,
  cancellation_token: CancellationToken,
  next_id: AtomicUsize,
}

impl Default for Dispatcher {
  fn default() -> Self {
    Self::new(1024)
  }
}

impl Dispatcher {
  pub fn new(capacity: usize) -> Self {
    let listeners = Arc::new(DashMap::<TypeId, Vec<ListenerWrapper>>::new());
    let listeners_cloned = listeners.clone();
    let async_listeners =
      Arc::new(DashMap::<TypeId, Vec<AsyncListenerWrapper>>::new());
    let async_listeners_cloned = async_listeners.clone();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(capacity);
    let cancellation_token = CancellationToken::new();
    let cancellation_token_cloned = cancellation_token.clone();

    tokio::spawn(async move {
      let listeners = listeners_cloned;
      let async_listeners = async_listeners_cloned;

      loop {
        let res: Option<Box<dyn Event>> = select! {
          message = receiver.recv() => message,
          _ = cancellation_token_cloned.cancelled() => break
        };

        if let Some(msg) = res {
          let type_id = Event::type_id(&*msg);
          if let Some(list) = listeners.get(&type_id) {
            for item in list.iter() {
              let _ = item.execute(msg.as_ref());
            }
          } else {
            continue;
          }

          if let Some(list) = async_listeners.get(&type_id) {
            for item in list.iter() {
              let _ = item.execute(msg.as_ref()).await;
            }
          } else {
            continue;
          }
        } else {
          break;
        }
      }
    });

    Self {
      sender,
      listeners,
      async_listeners,
      cancellation_token,
      next_id: AtomicUsize::new(0),
    }
  }

  pub fn install(self) -> Arc<Self> {
    let this = Arc::new(self);

    if DISPATCHER.set(this.clone()).is_err() {
      panic!("Dispatcher is installed twice")
    }

    this
  }

  pub fn get_instance() -> Arc<Self> {
    if let Some(this) = DISPATCHER.get() {
      this.clone()
    } else {
      panic!("Dispatcher is not installed yet")
    }
  }

  pub fn try_get_instance() -> Option<Arc<Self>> {
    DISPATCHER.get().cloned()
  }

  #[must_use = "ListenerId should be used because cancellation needs it"]
  pub fn on<E: Event, F>(&self, listener: F) -> ListenerId
  where
    F: Fn(&E) + Send + Sync + 'static,
  {
    let type_id = TypeId::of::<E>();

    let listener_id =
      ListenerId { type_id, id: self.next_id.fetch_add(1, Ordering::Release) };

    if let Some(mut list) = self.listeners.get_mut(&type_id) {
      list.push(ListenerWrapper::new(
        move |event: &E| {
          listener(event);
          Ok(())
        },
        listener_id,
      ));
    } else {
      self.listeners.insert(
        type_id,
        vec![ListenerWrapper::new(
          move |event: &E| {
            listener(event);
            Ok(())
          },
          listener_id,
        )],
      );
    }

    listener_id
  }

  #[must_use = "ListenerId should be used because cancellation needs it"]
  pub fn register<E: Event, L: Listener<E>>(&self, listener: L) -> ListenerId {
    let type_id = TypeId::of::<E>();

    let listener_id =
      ListenerId { type_id, id: self.next_id.fetch_add(1, Ordering::Release) };

    if let Some(mut list) = self.listeners.get_mut(&type_id) {
      list.push(ListenerWrapper::new(listener, listener_id));
    } else {
      self
        .listeners
        .insert(type_id, vec![ListenerWrapper::new(listener, listener_id)]);
    }

    listener_id
  }

  #[must_use = "ListenerId should be used because cancellation needs it"]
  pub fn register_async<E: Event, L: AsyncListener<E>>(
    &self, listener: L,
  ) -> ListenerId {
    let type_id = TypeId::of::<E>();

    let listener_id =
      ListenerId { type_id, id: self.next_id.fetch_add(1, Ordering::Release) };

    if let Some(mut list) = self.async_listeners.get_mut(&type_id) {
      list.push(AsyncListenerWrapper::new(listener, listener_id));
    } else {
      self.async_listeners.insert(
        type_id,
        vec![AsyncListenerWrapper::new(listener, listener_id)],
      );
    }

    listener_id
  }

  pub fn unregister(&self, id: ListenerId) {
    if let Some(mut list) = self.listeners.get_mut(&id.type_id) {
      list.pop_if(|item| item.id == id);
    } else if let Some(mut list) = self.async_listeners.get_mut(&id.type_id) {
      list.pop_if(|item| item.id == id);
    }
  }

  pub fn emit<E: Event>(&self, event: E) {
    let _ = self.sender.try_send(Box::new(event));
  }
}

impl Drop for Dispatcher {
  fn drop(&mut self) {
    self.cancellation_token.cancel();
  }
}
