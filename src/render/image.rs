use std::{
    any::Any,
    sync::{Arc, mpsc},
};

use wgpu::{
    BufferDescriptor, BufferUsages, COPY_BYTES_PER_ROW_ALIGNMENT, CommandEncoderDescriptor, Device,
    Extent3d, MapMode, PollType, Queue, TexelCopyBufferInfo, TexelCopyBufferLayout, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages,
    util::{DeviceExt, TextureDataOrder},
};

use crate::prelude::*;

/// Pixel layout the engine understands for CPU ⇄ GPU interchange.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PixelFormat {
    Rgba8Unorm,
    Rgba8UnormSrgb,
    Gray8,
}

impl PixelFormat {
    #[inline]
    pub const fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8Unorm | Self::Rgba8UnormSrgb => 4,
            Self::Gray8 => 1,
        }
    }

    #[inline]
    pub const fn to_wgpu(self) -> TextureFormat {
        match self {
            Self::Rgba8Unorm => TextureFormat::Rgba8Unorm,
            Self::Rgba8UnormSrgb => TextureFormat::Rgba8UnormSrgb,
            Self::Gray8 => TextureFormat::R8Unorm,
        }
    }

    #[inline]
    pub const fn from_wgpu(format: TextureFormat) -> Option<Self> {
        match format {
            TextureFormat::Rgba8Unorm => Some(Self::Rgba8Unorm),
            TextureFormat::Rgba8UnormSrgb => Some(Self::Rgba8UnormSrgb),
            TextureFormat::R8Unorm => Some(Self::Gray8),
            _ => None,
        }
    }
}

/// CPU-side image buffer with explicit format metadata.
#[derive(Debug, Clone)]
pub struct RawImage {
    width: u32,
    height: u32,
    format: PixelFormat,
    data: Arc<[u8]>,
}

impl RawImage {
    /// Construct a new image from raw bytes.
    pub fn from_bytes(
        format: PixelFormat,
        width: u32,
        height: u32,
        bytes: impl Into<Vec<u8>>,
    ) -> Result<Self> {
        let bytes = bytes.into();
        let expected = width as usize * height as usize * format.bytes_per_pixel();
        if bytes.len() != expected {
            return Err(LunarisError::InvalidArgument {
                name: "image bytes".to_string(),
                reason: Some(format!(
                    "expected {} bytes for {}x{} {:?}, got {}",
                    expected,
                    width,
                    height,
                    format,
                    bytes.len()
                )),
            });
        }

        Ok(Self {
            width,
            height,
            format,
            data: Arc::from(bytes.into_boxed_slice()),
        })
    }

    /// Convenience constructor for linear RGBA8 images.
    pub fn from_rgba8(width: u32, height: u32, bytes: impl Into<Vec<u8>>) -> Result<Self> {
        Self::from_bytes(PixelFormat::Rgba8Unorm, width, height, bytes)
    }

    /// Convenience constructor for sRGB RGBA8 images.
    pub fn from_rgba8_srgb(width: u32, height: u32, bytes: impl Into<Vec<u8>>) -> Result<Self> {
        Self::from_bytes(PixelFormat::Rgba8UnormSrgb, width, height, bytes)
    }

    /// Zero-filled image for the given format.
    pub fn zeroed(format: PixelFormat, width: u32, height: u32) -> Self {
        let len = width as usize * height as usize * format.bytes_per_pixel();
        Self {
            width,
            height,
            format,
            data: Arc::from(vec![0; len].into_boxed_slice()),
        }
    }

    #[inline]
    pub const fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub const fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub const fn format(&self) -> PixelFormat {
        self.format
    }

    #[inline]
    pub fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    #[inline]
    pub fn len(&self) -> usize {
        self.data.len()
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    #[inline]
    pub fn bytes_per_pixel(&self) -> usize {
        self.format.bytes_per_pixel()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    pub fn into_bytes(self) -> Arc<[u8]> {
        self.data
    }

    fn ensure_geometry(&self, other: &Self) -> Result<()> {
        if self.width != other.width || self.height != other.height {
            return Err(LunarisError::Dimensionmismatch {
                a: (self.width as usize, self.height as usize),
                b: (other.width as usize, other.height as usize),
            });
        }
        if self.format != other.format {
            return Err(LunarisError::InvalidArgument {
                name: "image format".to_string(),
                reason: Some("pixel format mismatch".to_string()),
            });
        }
        Ok(())
    }

    /// Saturating overlay of two images with identical geometry.
    pub fn overlay(&self, other: &Self) -> Result<Self> {
        self.ensure_geometry(other)?;
        let mut data = Vec::with_capacity(self.len());
        data.extend(
            self.as_bytes()
                .iter()
                .zip(other.as_bytes().iter())
                .map(|(a, b)| a.saturating_add(*b)),
        );

        Self::from_bytes(self.format, self.width, self.height, data)
    }

    /// Downsample by 2x using a simple box filter. For odd dimensions the
    /// remaining row/column is averaged with the available neighbours.
    pub fn size_down(&self) -> Self {
        let bpp = self.bytes_per_pixel();
        let new_width = self.width.max(1).div_ceil(2);
        let new_height = self.height.max(1).div_ceil(2);
        let mut out = vec![0u8; new_width as usize * new_height as usize * bpp];
        let src = self.as_bytes();

        for y in 0..new_height {
            for x in 0..new_width {
                for c in 0..bpp {
                    let mut sum = 0u32;
                    let mut count = 0u32;
                    for dy in 0..2 {
                        let sy = y * 2 + dy;
                        if sy >= self.height {
                            continue;
                        }
                        for dx in 0..2 {
                            let sx = x * 2 + dx;
                            if sx >= self.width {
                                continue;
                            }
                            let idx = ((sy * self.width + sx) as usize) * bpp + c;
                            sum += src[idx] as u32;
                            count += 1;
                        }
                    }
                    let dst_idx = ((y * new_width + x) as usize) * bpp + c;
                    out[dst_idx] = (sum / count.max(1)) as u8;
                }
            }
        }

        Self {
            width: new_width,
            height: new_height,
            format: self.format,
            data: Arc::from(out.into_boxed_slice()),
        }
    }

    /// Upload the image into a GPU texture with the desired usage flags.
    pub fn to_texture(&self, device: &Device, queue: &Queue, usage: TextureUsages) -> Texture {
        let desc = TextureDescriptor {
            label: Some("RawImage"),
            size: self.extent(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: self.format.to_wgpu(),
            usage: usage | TextureUsages::COPY_DST,
            view_formats: &[],
        };

        device.create_texture_with_data(queue, &desc, TextureDataOrder::LayerMajor, self.as_bytes())
    }

    #[inline]
    pub fn extent(&self) -> Extent3d {
        Extent3d {
            width: self.width,
            height: self.height,
            depth_or_array_layers: 1,
        }
    }

    pub fn compress(self, strategy: CompressionStrategy) -> Result<CompressedImage> {
        let compressed: Vec<u8> = match strategy {
            CompressionStrategy::Raw => self.data.as_ref().to_vec(),
            CompressionStrategy::Qoi => qoi::encode_to_vec(self.data, self.width, self.height)
                .map_err(|e| LunarisError::FailedCompress {
                    what: e.to_string(),
                })?,
            CompressionStrategy::Lz4 => lz4_flex::block::compress_prepend_size(&self.data),
            CompressionStrategy::Zstd(level) => zstd::encode_all(&*self.data, level as i32)
                .map_err(|e| LunarisError::FailedCompress {
                    what: e.to_string(),
                })?,
        };
        Ok(CompressedImage {
            width: self.width,
            height: self.height,
            format: self.format,
            codec: strategy,
            payload: compressed,
        })
    }
}

pub struct CompressedImage {
    width: u32,
    height: u32,
    format: PixelFormat,
    codec: CompressionStrategy,
    payload: Vec<u8>,
}

#[derive(Clone, Copy)]
pub enum CompressionStrategy {
    Raw,
    Zstd(u8),
    Qoi,
    Lz4,
}

impl CompressedImage {
    pub fn decompress(&self) -> Result<RawImage> {
        let expected = self.width as usize * self.height as usize * self.format.bytes_per_pixel();

        let data = match self.codec {
            CompressionStrategy::Raw => self.payload.clone(),
            CompressionStrategy::Qoi => {
                let (header, decoded) = qoi::decode_to_vec(&self.payload).map_err(|e| {
                    LunarisError::FailedDecompress {
                        what: e.to_string(),
                    }
                })?;

                if header.width != self.width || header.height != self.height {
                    return Err(LunarisError::InvalidArgument {
                        name: "qoi header".to_string(),
                        reason: Some("dimension mismatch".to_string()),
                    });
                }

                if header.channels.as_u8() as usize != self.format.bytes_per_pixel() {
                    return Err(LunarisError::InvalidArgument {
                        name: "qoi header".to_string(),
                        reason: Some("channel count mismatch".to_string()),
                    });
                }

                decoded
            }
            CompressionStrategy::Lz4 => lz4_flex::block::decompress_size_prepended(&self.payload)
                .map_err(|e| LunarisError::FailedDecompress {
                what: e.to_string(),
            })?,
            CompressionStrategy::Zstd(_) => {
                zstd::decode_all(&*self.payload).map_err(|e| LunarisError::FailedDecompress {
                    what: e.to_string(),
                })?
            }
        };

        if data.len() != expected {
            return Err(LunarisError::InvalidArgument {
                name: "decompressed image".to_string(),
                reason: Some(format!("expected {} bytes, got {}", expected, data.len())),
            });
        }

        RawImage::from_bytes(self.format, self.width, self.height, data)
    }
}

fn read_texture_into_raw(texture: &Texture) -> RawImage {
    assert_eq!(
        texture.dimension(),
        TextureDimension::D2,
        "only 2D textures are supported"
    );
    assert!(
        texture.usage().contains(TextureUsages::COPY_SRC),
        "texture missing COPY_SRC usage required for readback"
    );

    let size = texture.size();
    let format = PixelFormat::from_wgpu(texture.format())
        .expect("unsupported texture format for RawImage conversion");

    if size.width == 0 || size.height == 0 {
        return RawImage::zeroed(format, size.width, size.height);
    }

    let bytes_per_pixel = format.bytes_per_pixel();
    let bytes_per_row = bytes_per_pixel
        .checked_mul(size.width as usize)
        .expect("row byte count overflow");
    let alignment = COPY_BYTES_PER_ROW_ALIGNMENT as usize;
    let padded_bytes_per_row = if bytes_per_row == 0 {
        alignment
    } else {
        ((bytes_per_row + alignment - 1) / alignment) * alignment
    };
    let padded_bytes_per_row_u32 =
        u32::try_from(padded_bytes_per_row).expect("row stride exceeds u32::MAX");

    let buffer_size = padded_bytes_per_row
        .checked_mul(size.height as usize)
        .expect("buffer size overflow");
    let buffer = super::device().create_buffer(&BufferDescriptor {
        label: Some("RawImage staging buffer"),
        size: buffer_size as u64,
        usage: BufferUsages::MAP_READ | BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = super::device().create_command_encoder(&CommandEncoderDescriptor {
        label: Some("RawImage readback encoder"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        TexelCopyBufferInfo {
            buffer: &buffer,
            layout: TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(padded_bytes_per_row_u32),
                rows_per_image: Some(size.height),
            },
        },
        Extent3d {
            width: size.width,
            height: size.height,
            depth_or_array_layers: 1,
        },
    );

    super::queue().submit([encoder.finish()]);

    let buffer_slice = buffer.slice(..);
    let (sender, receiver) = mpsc::channel();
    buffer_slice.map_async(MapMode::Read, move |result| {
        let _ = sender.send(result);
    });

    super::device()
        .poll(PollType::Wait)
        .expect("failed to poll device for texture readback");
    receiver
        .recv()
        .expect("failed to receive GPU map result")
        .expect("failed to map texture buffer for readback");

    let mapped = buffer_slice.get_mapped_range();
    let mut pixels = Vec::with_capacity(bytes_per_row * size.height as usize);
    let row_pitch = padded_bytes_per_row;
    for chunk in mapped.chunks(row_pitch).take(size.height as usize) {
        pixels.extend_from_slice(&chunk[..bytes_per_row]);
    }
    drop(mapped);
    buffer.unmap();

    RawImage::from_bytes(format, size.width, size.height, pixels)
        .expect("texture readback produced invalid data")
}

impl From<RawImage> for CompressedImage {
    fn from(image: RawImage) -> Self {
        let RawImage {
            width,
            height,
            format,
            data,
        } = image;

        Self {
            width,
            height,
            format,
            codec: CompressionStrategy::Raw,
            payload: data.to_vec(),
        }
    }
}

impl From<&RawImage> for CompressedImage {
    fn from(image: &RawImage) -> Self {
        Self {
            width: image.width,
            height: image.height,
            format: image.format,
            codec: CompressionStrategy::Raw,
            payload: image.data.as_ref().to_vec(),
        }
    }
}

impl From<&CompressedImage> for RawImage {
    fn from(image: &CompressedImage) -> Self {
        image
            .decompress()
            .expect("compressed image could not be decompressed")
    }
}

impl From<CompressedImage> for RawImage {
    fn from(image: CompressedImage) -> Self {
        (&image).into()
    }
}

impl From<&RawImage> for Texture {
    fn from(image: &RawImage) -> Self {
        image.to_texture(
            super::device(),
            super::queue(),
            TextureUsages::TEXTURE_BINDING | TextureUsages::COPY_SRC,
        )
    }
}

impl From<RawImage> for Texture {
    fn from(image: RawImage) -> Self {
        (&image).into()
    }
}

impl From<&Texture> for RawImage {
    fn from(texture: &Texture) -> Self {
        read_texture_into_raw(texture)
    }
}

impl From<Texture> for RawImage {
    fn from(texture: Texture) -> Self {
        read_texture_into_raw(&texture)
    }
}

impl From<&CompressedImage> for Texture {
    fn from(image: &CompressedImage) -> Self {
        RawImage::from(image).into()
    }
}

impl From<CompressedImage> for Texture {
    fn from(image: CompressedImage) -> Self {
        RawImage::from(image).into()
    }
}

impl From<&Texture> for CompressedImage {
    fn from(texture: &Texture) -> Self {
        RawImage::from(texture).into()
    }
}

impl From<Texture> for CompressedImage {
    fn from(texture: Texture) -> Self {
        RawImage::from(texture).into()
    }
}

pub enum RenderResult {
    RawImage(RawImage),
    Number(u64),
    Waveform(Vec<f64>),
    Other(Box<dyn Any + Send + 'static>),
}
