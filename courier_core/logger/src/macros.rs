#[macro_export]
macro_rules! trace {
  (namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().trace_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().trace_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().trace_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().trace(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().trace(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().trace(format!($template, $($key = $value),*), $namespace);
  };
}

#[macro_export]
macro_rules! debug {
  (namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().debug_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().debug_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().debug_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().debug(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().debug(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().debug(format!($template, $($key = $value),*), $namespace);
  };
}

#[macro_export]
macro_rules! info {
  (namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().info_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().info_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().info_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().info_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().info_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().info_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().info_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().info(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().info(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().info(format!($template, $($key = $value),*), $namespace);
  };
}

#[macro_export]
macro_rules! warn {
  (namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().warn_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().warn_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().warn_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().warn(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().warn(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().warn(format!($template, $($key = $value),*), $namespace);
  };
}

#[macro_export]
macro_rules! error {
  (namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().error_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().error_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().error_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().error_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().error_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().error_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().error_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().error(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().error(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().error(format!($template, $($key = $value),*), $namespace);
  };
}

#[macro_export]
macro_rules! fatal {
  (namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace: $namespace: literal, msg: $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().fatal_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().fatal_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().fatal_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace: $namespace: literal, template: $template: literal, args: $($args:tt),*) => {
    $crate::Logger::get_instance().fatal(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().fatal(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().fatal(format!($template, $($key = $value),*), $namespace);
  };
}
