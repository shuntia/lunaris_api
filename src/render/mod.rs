use crate::prelude::*;

#[derive(Debug)]
pub struct RawImage {
    pub size: (usize, usize),
    pub frame: Vec<u8>,
}

impl RawImage {
    pub fn is_valid(&self) -> bool {
        self.frame.len() == self.size.0 * self.size.1
    }
    pub fn overlay(mut self, other: RawImage) -> Result<Self> {
        if self.size != other.size {
            Err(LunarisError::Dimensionmismatch {
                a: self.size,
                b: other.size,
            })
        } else {
            self.frame
                .iter_mut()
                .zip(other.frame.iter())
                .for_each(|(a, b)| *a = a.saturating_add(*b));
            Ok(self)
        }
    }
}
