use std::{
  any::TypeId,
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicUsize, Ordering},
  },
};

use dashmap::DashMap;
use tokio::{select, sync::mpsc::Sender, task::JoinHandle};
use tokio_util::sync::CancellationToken;

use crate::{
  Event,
  listener::{
    AsyncListener, AsyncListenerWrapper, Listener, ListenerId, ListenerWrapper,
  },
};

static DISPATCHER: OnceLock<Arc<Dispatcher>> = OnceLock::new();

pub struct Dispatcher {
  /// The send half of the event channel. It is taken by
  /// [`shutdown`](Self::shutdown), which closes the channel and lets the
  /// event loop drain the remaining events before exiting.
  sender: Mutex<Option<Sender<Box<dyn Event>>>>,
  listeners: Arc<DashMap<TypeId, Vec<ListenerWrapper>>>,
  async_listeners: Arc<DashMap<TypeId, Vec<AsyncListenerWrapper>>>,
  cancellation_token: CancellationToken,
  /// Handle of the event loop task, joined by
  /// [`shutdown`](Self::shutdown).
  handle: Mutex<Option<JoinHandle<()>>>,
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

    let handle = tokio::spawn(async move {
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
          }

          if let Some(list) = async_listeners.get(&type_id) {
            for item in list.iter() {
              let _ = item.execute(msg.as_ref()).await;
            }
          }
        } else {
          break;
        }
      }
    });

    Self {
      sender: Mutex::new(Some(sender)),
      listeners,
      async_listeners,
      cancellation_token,
      handle: Mutex::new(Some(handle)),
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
    if let Ok(sender) = self.sender.lock()
      && let Some(sender) = sender.as_ref()
    {
      let _ = sender.try_send(Box::new(event));
    }
  }

  /// Stops the event loop.
  ///
  /// Events that are still queued are delivered to their listeners
  /// before the background task exits, and this only returns once that
  /// task has finished, so nothing is lost while shutting down. Events
  /// emitted afterwards are silently dropped.
  ///
  /// **Note**: do not call this from inside a listener — it waits for
  /// the very task that runs the listeners.
  pub async fn shutdown(&self) {
    // Dropping the sender closes the channel: the loop delivers the
    // events that are still queued and then exits on its own.
    if let Ok(mut sender) = self.sender.lock() {
      sender.take();
    }

    let handle = self.handle.lock().ok().and_then(|mut handle| handle.take());

    if let Some(handle) = handle {
      let _ = handle.await;
    }
  }
}

impl Drop for Dispatcher {
  fn drop(&mut self) {
    self.cancellation_token.cancel();
  }
}
