use std::{
  borrow::Borrow,
  ops::{Deref, RangeBounds},
};

use bytes::Bytes;

/// A cheaply cloneable, reference-counted byte payload.
///
/// `Payload` wraps [`bytes::Bytes`] so that callers can build and consume
/// byte data without depending on the `bytes` crate themselves. Cloning a
/// payload is constant-time, and slicing or splitting shares the underlying
/// allocation instead of copying it.
///
/// Converting from an owned [`Vec<u8>`] or [`String`] reuses its buffer,
/// while converting from a borrowed slice or [`str`] copies the data; use
/// [`from_static`](Self::from_static) for static data.
///
/// # Examples
///
/// ```rust
/// use protocol::Payload;
///
/// let mut payload = Payload::from_static(b"hello world");
/// assert_eq!(payload.len(), 11);
///
/// let tail = payload.split_off(6);
/// assert_eq!(&payload[..], b"hello ");
/// assert_eq!(&tail[..], b"world");
/// ```
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct Payload(Bytes);

impl Payload {
  /// Creates an empty payload.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// let payload = Payload::new();
  /// assert!(payload.is_empty());
  /// ```
  pub const fn new() -> Self {
    Self(Bytes::new())
  }

  /// Creates a payload pointing at `bytes` without copying.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// let payload = Payload::from_static(b"hello");
  /// assert_eq!(&payload[..], b"hello");
  /// ```
  pub const fn from_static(bytes: &'static [u8]) -> Self {
    Self(Bytes::from_static(bytes))
  }

  /// Returns the number of bytes in the payload.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// assert_eq!(Payload::from_static(b"hello").len(), 5);
  /// ```
  pub const fn len(&self) -> usize {
    self.0.len()
  }

  /// Returns `true` if the payload contains no bytes.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// assert!(Payload::new().is_empty());
  /// assert!(!Payload::from_static(b"hello").is_empty());
  /// ```
  pub const fn is_empty(&self) -> bool {
    self.0.is_empty()
  }

  /// Returns a new payload sharing the same allocation, restricted to
  /// `range`.
  ///
  /// # Panics
  ///
  /// Panics if `range` is out of bounds.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// let payload = Payload::from_static(b"hello world");
  /// assert_eq!(&payload.slice(6..)[..], b"world");
  /// ```
  pub fn slice(&self, range: impl RangeBounds<usize>) -> Self {
    Self(self.0.slice(range))
  }

  /// Splits the payload into two at `at`, returning the tail.
  ///
  /// # Panics
  ///
  /// Panics if `at` is greater than the payload length.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// let mut payload = Payload::from_static(b"hello world");
  /// let tail = payload.split_off(6);
  /// assert_eq!(&payload[..], b"hello ");
  /// assert_eq!(&tail[..], b"world");
  /// ```
  pub fn split_off(&mut self, at: usize) -> Self {
    Self(self.0.split_off(at))
  }

  /// Splits off the first `at` bytes, returning them as a new payload.
  ///
  /// # Panics
  ///
  /// Panics if `at` is greater than the payload length.
  ///
  /// # Examples
  ///
  /// ```rust
  /// use protocol::Payload;
  ///
  /// let mut payload = Payload::from_static(b"hello world");
  /// let head = payload.split_to(6);
  /// assert_eq!(&head[..], b"hello ");
  /// assert_eq!(&payload[..], b"world");
  /// ```
  pub fn split_to(&mut self, at: usize) -> Self {
    Self(self.0.split_to(at))
  }
}

impl Deref for Payload {
  type Target = [u8];

  fn deref(&self) -> &Self::Target {
    &self.0
  }
}

impl AsRef<[u8]> for Payload {
  fn as_ref(&self) -> &[u8] {
    &self.0
  }
}

impl Borrow<[u8]> for Payload {
  fn borrow(&self) -> &[u8] {
    &self.0
  }
}

impl From<Vec<u8>> for Payload {
  fn from(value: Vec<u8>) -> Self {
    Self(Bytes::from(value))
  }
}

impl From<String> for Payload {
  fn from(value: String) -> Self {
    Self(Bytes::from(value))
  }
}

impl<'a> From<&'a [u8]> for Payload {
  fn from(value: &'a [u8]) -> Self {
    Self(Bytes::copy_from_slice(value))
  }
}

impl<'a> From<&'a str> for Payload {
  fn from(value: &'a str) -> Self {
    Self(Bytes::copy_from_slice(value.as_bytes()))
  }
}

impl From<Bytes> for Payload {
  fn from(value: Bytes) -> Self {
    Self(value)
  }
}

impl From<Payload> for Bytes {
  fn from(value: Payload) -> Self {
    value.0
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn splits_share_the_original_allocation() {
    let mut payload = Payload::from(Vec::from("hello world"));
    let tail = payload.split_off(6);

    assert_eq!(&payload[..], b"hello ");
    assert_eq!(&tail[..], b"world");
    assert_eq!(payload.as_ptr().wrapping_add(payload.len()), tail.as_ptr(),);
  }

  #[test]
  fn conversions_from_owned_and_borrowed_sources() {
    assert_eq!(&Payload::from(vec![1_u8, 2, 3])[..], &[1, 2, 3]);
    assert_eq!(&Payload::from(String::from("abc"))[..], b"abc");
    assert_eq!(&Payload::from("abc")[..], b"abc");
    assert_eq!(&Payload::from(&b"abc"[..])[..], b"abc");
    assert_eq!(&Payload::from("abc".to_owned().as_str())[..], b"abc");
  }
}
