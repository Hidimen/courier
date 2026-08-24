use network::transport::Transport;

pub trait NetworkEnhancement<T: Transport> {
  fn get() -> T;
}