use relay::{Layer, Middleware};

pub trait MiddlewareEnhancement<Req, M: Middleware<Req>> {
  fn get() -> M;
}

pub trait LayerEnhancement<Inner, L: Layer<Inner>> {
  fn get() -> L;
}