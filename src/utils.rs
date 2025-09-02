pub trait Utils {
  fn ok<E>(self) -> Result<Self, E>
  where
    Self: Sized,
  {
    Ok(self)
  }

  fn some(self) -> Option<Self>
  where
    Self: Sized,
  {
    Some(self)
  }

  fn unit(&self) {}
}

impl<T: ?Sized> Utils for T {}
