use crate::message::Message;

#[allow(unused)]
pub trait Format {
  fn format(&self, buf: &mut dyn std::io::Write, msg: Message);
}
