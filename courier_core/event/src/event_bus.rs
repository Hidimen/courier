use std::{
  any::TypeId,
  sync::{
    Arc, Mutex, OnceLock,
    atomic::{AtomicBool, Ordering},
  },
  thread::JoinHandle,
  time::Duration,
};

use crossbeam_channel::{RecvTimeoutError, Sender, TrySendError};
use dashmap::DashMap;

use crate::{
  Event, EventHandler, Subscriber, Subscription,
  error::{
    EmitError, RegisterError, SubscribeError, UnregisterError, UnsubscribeError,
  },
};

static EVENT_BUS: OnceLock<Arc<EventBus>> = OnceLock::new();

pub struct EventBus {
  handle: Mutex<Option<JoinHandle<()>>>,
  signal: Arc<AtomicBool>,
  sender: Sender<Box<dyn Event>>,
  subscribers: Arc<DashMap<TypeId, Vec<EventHandler>>>,
}

impl Default for EventBus {
  fn default() -> Self {
    Self::new(1024)
  }
}

impl EventBus {
  pub fn new(capacity: usize) -> Self {
    let signal = Arc::new(AtomicBool::new(true));
    let signal_cloned = signal.clone();
    let (sender, receiver) =
      crossbeam_channel::bounded::<Box<dyn Event>>(capacity);
    let subscribers = Arc::new(DashMap::<TypeId, Vec<EventHandler>>::new());
    let subscribers_cloned = subscribers.clone();

    let handle = std::thread::Builder::new()
      .name("courier:event".into())
      .stack_size(3 * 1024 * 1024)
      .spawn(move || {
        let signal = signal_cloned;
        let receiver = receiver;
        let subscribers = subscribers_cloned;

        while signal.load(Ordering::Acquire) {
          match receiver.recv_timeout(Duration::from_millis(500)) {
            Ok(event) => {
              let type_id = (*event).type_id();

              if let Some(handlers) = subscribers.get(&type_id) {
                for handler in handlers.iter() {
                  let handler: &EventHandler = handler;
                  handler.execute(event.as_ref());
                }
              }
            },
            Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => continue,
          }
        }
      })
      .unwrap();

    Self { handle: Mutex::new(Some(handle)), signal, sender, subscribers }
  }

  pub fn install(self) -> Arc<Self> {
    let this = Arc::new(self);
    if EVENT_BUS.set(this.clone()).is_err() {
      panic!("EventBus has been initialized");
    }

    this
  }

  pub fn register<E: Event>(&self) -> Result<(), RegisterError> {
    let type_id = TypeId::of::<E>();
    if self.subscribers.contains_key(&type_id) {
      Err(RegisterError::EventAlreadyExists)
    } else {
      self.subscribers.insert(type_id, Vec::new());
      Ok(())
    }
  }

  pub fn unregister<E: Event>(&self) -> Result<(), UnregisterError> {
    let type_id = TypeId::of::<E>();

    if self.subscribers.remove(&type_id).is_none() {
      Err(UnregisterError::EventNotFound)
    } else {
      Ok(())
    }
  }

  pub fn emit<E: Event>(&self, event: E) -> Result<(), EmitError> {
    match self.sender.try_send(Box::new(event)) {
      Ok(_) => Ok(()),
      Err(TrySendError::Full(_)) => Err(EmitError::Full),
      Err(TrySendError::Disconnected(_)) => Err(EmitError::Closed),
    }
  }

  pub fn subscribe<S: Subscriber<E>, E: Event>(
    &self, subscriber: S,
  ) -> Result<Subscription, SubscribeError> {
    let type_id = TypeId::of::<E>();

    if let Some(mut list) = self.subscribers.get_mut(&type_id) {
      let name = subscriber.name();
      list.push(EventHandler::new(subscriber));
      Ok(Subscription { type_id, name })
    } else {
      Err(SubscribeError::EventNotFound)
    }
  }

  pub fn unsubscribe(
    &self, subscription: Subscription,
  ) -> Result<(), UnsubscribeError> {
    let type_id = subscription.type_id;
    let name = subscription.name;

    if let Some(mut list) = self.subscribers.get_mut(&type_id) {
      let old_len = list.len();
      list.retain(|ele| ele.name() != name);
      if list.len() != old_len {
        Ok(())
      } else {
        Err(UnsubscribeError::SubscriberNotFound)
      }
    } else {
      Err(UnsubscribeError::EventNotFound)
    }
  }

  pub fn get_instance() -> Arc<Self> {
    match EVENT_BUS.get() {
      Some(s) => s.clone(),
      None => panic!("EventBus not initialized"),
    }
  }
}

impl Drop for EventBus {
  fn drop(&mut self) {
    self.signal.store(false, Ordering::Release);
    if let Ok(mut guard) = self.handle.lock()
      && let Some(handle) = guard.take()
    {
      let _ = handle.join();
    }
  }
}
