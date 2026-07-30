#[macro_export]
macro_rules! event {
  ($event: tt) => {
    $crate::EventBus::get_instance().emit($event)
  };
}
