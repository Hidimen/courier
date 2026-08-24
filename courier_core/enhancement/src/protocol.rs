use protocol::Protocol;

pub trait ProtocolEnhancement<P: Protocol> {
  fn get() -> P;
}