#[macro_export]
macro_rules! event {
  ($event: ident) => {
    $crate::Dispatcher::get_instance().emit($event)
  };
  ($name:ident { $($field:ident : $value:expr),* $(,)? }) => {
    $crate::Dispatcher::get_instance().emit($name {
      $($field: $value)*
    })
  };
}
