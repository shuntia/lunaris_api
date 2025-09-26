use std::sync::OnceLock;

use wgpu::{Device, Queue};

use crate::prelude::*;

pub mod cache;
pub mod image;

pub use image::{PixelFormat, RawImage, RenderResult};

pub static DEVICE: OnceLock<Device> = OnceLock::new();
pub static QUEUE: OnceLock<Queue> = OnceLock::new();

/// Register the globally shared GPU handles. Must be called once by the host
/// during startup before any render helpers are used.
pub fn init_gpu(device: Device, queue: Queue) -> Result {
    if DEVICE.set(device).is_err() {
        return Err(LunarisError::AlreadyExists {
            item: "wgpu device".to_string(),
        });
    }

    if QUEUE.set(queue).is_err() {
        return Err(LunarisError::AlreadyExists {
            item: "wgpu queue".to_string(),
        });
    }

    Ok(())
}

/// Clone the global device handle. Panics if [`init_gpu`] has not been called.
pub fn device() -> &'static Device {
    DEVICE.get().expect("GPU device not initialized")
}

/// Clone the global queue handle. Panics if [`init_gpu`] has not been called.
pub fn queue() -> &'static Queue {
    QUEUE.get().expect("GPU queue not initialized")
}
