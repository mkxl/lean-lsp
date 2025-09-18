pub trait Is<T> {
  fn take(self) -> T;
}

impl<T> Is<T> for T {
  fn take(self) -> T {
    self
  }
}
