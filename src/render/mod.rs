#[derive(Debug)]
pub struct RawImage {
    pub size: (usize, usize),
    pub frame: Vec<u8>,
}

impl RawImage {
    pub fn is_valid(&self) -> bool {
        self.frame.len() == self.size.0 * self.size.1
    }
}
