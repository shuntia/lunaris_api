pub trait ShareableState: Send + Sync + 'static {
    fn source(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn serialize(&self, bytes: &mut [u8]);
    fn deserialize(bytes: &[u8]) -> Self
    where
        Self: Sized;
}
