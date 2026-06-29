/// Logs a message at the [`Trace`](crate::Level::Trace) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed via
/// [`Builder::build`](crate::Builder::build).
///
/// # Usage
///
/// ```rust,ignore
/// // Requires a globally-installed logger.
/// trace!("my_mod", "Trace message");
/// trace!(namespace = "my_mod", msg = "Trace message");
/// trace!(target = "file", "my_mod", "Trace with target");
/// trace!(target = "file", namespace = "my_mod", msg = "Trace with target");
/// trace!("my_mod", "x = {}", x);
/// trace!(namespace = "my_mod", template = "x = {}", args = x);
/// ```
///
/// **Note**: please make sure that `target` name is existing, provided by
/// specific [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! trace {
  (namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().trace_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().trace_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().trace_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().trace_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().trace(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().trace(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().trace(format!($template, $($key = $value),*), $namespace);
  };
}

/// Logs a message at the [`Debug`](crate::Level::Debug) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,ignore
/// // Requires a globally-installed logger.
/// debug!("my_mod", "Debug message");
/// debug!(namespace = "my_mod", msg = "Debug message");
/// debug!(target = "file", "my_mod", "Debug with target");
/// debug!(target = "file", namespace = "my_mod", msg = "Debug with target");
/// debug!("my_mod", "x = {}", x);
/// debug!(namespace = "my_mod", template = "x = {}", args = x);
/// ```
///
/// **Note**: please make sure that `target` name is existing, provided by
/// specific [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! debug {
  (namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().debug_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().debug_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().debug_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().debug_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().debug(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().debug(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().debug(format!($template, $($key = $value),*), $namespace);
  };
}

/// Logs a message at the [`Info`](crate::Level::Info) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,ignore
/// // Requires a globally-installed logger.
/// info!("my_mod", "Info message");
/// info!(namespace = "my_mod", msg = "Info message");
/// info!(target = "file", "my_mod", "Info with target");
/// info!(target = "file", namespace = "my_mod", msg = "Info with target");
/// info!("my_mod", "value = {}", v);
/// info!(namespace = "my_mod", template = "value = {}", args = v);
/// ```
///
/// **Note**: please make sure that `target` name is existing, provided by
/// specific [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! info {
  (namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().info_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().info_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().info_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().info_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().info_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().info_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().info_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().info(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().info(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().info(format!($template, $($key = $value),*), $namespace);
  };
}

/// Logs a message at the [`Warn`](crate::Level::Warn) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,ignore
/// // Requires a globally-installed logger.
/// warn!("my_mod", "Warning message");
/// warn!(namespace = "my_mod", msg = "Warning message");
/// warn!(target = "file", "my_mod", "Warning with target");
/// warn!(target = "file", namespace = "my_mod", msg = "Warning with target");
/// warn!("my_mod", "low disk space: {}%", pct);
/// warn!(namespace = "my_mod", template = "low disk space: {}%", args = pct);
/// ```
///
/// **Note**: please make sure that `target` name is existing, provided by
/// specific [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! warn {
  (namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().warn_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().warn_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().warn_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().warn_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().warn(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().warn(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().warn(format!($template, $($key = $value),*), $namespace);
  };
}

/// Logs a message at the [`Error`](crate::Level::Error) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,ignore
/// // Requires a globally-installed logger.
/// error!("my_mod", "Error message");
/// error!(namespace = "my_mod", msg = "Error message");
/// error!(target = "file", "my_mod", "Error with target");
/// error!(target = "file", namespace = "my_mod", msg = "Error with target");
/// error!("my_mod", "io error: {}", e);
/// error!(namespace = "my_mod", template = "io error: {}", args = e);
/// ```
///
/// **Note**: please make sure that `target` name is existing, provided by
/// specific [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! error {
  (namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().error_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().error_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().error_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().error_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().error_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().error_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().error_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().error(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().error(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().error(format!($template, $($key = $value),*), $namespace);
  };
}

/// Logs a message at the [`Fatal`](crate::Level::Fatal) level via the
/// globally-installed logger.
///
/// # Panics
///
/// Panics if no logger has been installed.
///
/// # Usage
///
/// ```rust,ignore
/// // Requires a globally-installed logger.
/// fatal!("my_mod", "Fatal error");
/// fatal!(namespace = "my_mod", msg = "Fatal error");
/// fatal!(target = "file", "my_mod", "Fatal with target");
/// fatal!(target = "file", namespace = "my_mod", msg = "Fatal with target");
/// fatal!("my_mod", "cannot continue: {}", reason);
/// fatal!(namespace = "my_mod", template = "cannot continue: {}", args = reason);
/// ```
///
/// **Note**: please make sure that `target` name is existing, provided by
/// specific [`Flow`](crate::Flow) implementation.
#[macro_export]
macro_rules! fatal {
  (namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static($msg, $namespace);
  };

  ($namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static($msg, $namespace);
  };

  (target = $target:literal, namespace = $namespace: literal, msg = $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $msg: literal) => {
    $crate::Logger::get_instance().fatal_from_static_with_target($msg, $namespace, $target);
  };

  (target = $target:literal, namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().fatal_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().fatal_with_target(format!($template, $($args),*), $namespace, $target);
  };

  (target = $target:literal, $namespace: literal, $template: literal, $($key:tt = $value:tt),*) => {
    $crate::Logger::get_instance().fatal_with_target(format!($template, $($key = $value),*), $namespace, $target);
  };

  (namespace = $namespace: literal, template = $template: literal, args = $($args:tt),*) => {
    $crate::Logger::get_instance().fatal(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($args:tt),*) => {
    $crate::Logger::get_instance().fatal(format!($template, $($args),*), $namespace);
  };

  ($namespace: literal, $template: literal, $($key:ident = $value:tt),*) => {
    $crate::Logger::get_instance().fatal(format!($template, $($key = $value),*), $namespace);
  };
}
