use std::fmt::Display;

mod decoder;
/// Raw FFI bindings to libvpx.
#[allow(warnings)]
pub mod ffi;
pub mod ivf;

pub use decoder::Vp9Decoder;

#[derive(Debug)]
pub struct Error(u32);

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "codec error {}", self.0)
    }
}

/// A frame of YUV420 pixel data.
///
/// Can be reused to save on allocations.
#[derive(Debug)]
pub struct Frame {
    width: u32,
    height: u32,
    y_plane: Vec<u8>,
    u_plane: Vec<u8>,
    v_plane: Vec<u8>,
}

impl Frame {
    pub fn new(width: u32, height: u32) -> Self {
        let full_size = width as usize * height as usize;
        let half_size = (width / 2) as usize * (height / 2) as usize;
        Self {
            width,
            height,
            y_plane: vec![0u8; full_size],
            u_plane: vec![0u8; half_size],
            v_plane: vec![0u8; half_size],
        }
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn y_plane(&self) -> &[u8] {
        &self.y_plane
    }

    pub fn u_plane(&self) -> &[u8] {
        &self.u_plane
    }

    pub fn v_plane(&self) -> &[u8] {
        &self.v_plane
    }

    pub fn get_y(&self, x: u32, y: u32) -> u8 {
        self.y_plane[(x + y * self.width) as usize]
    }

    pub fn get_uv(&self, x: u32, y: u32) -> (u8, u8) {
        (
            self.u_plane[(x + y * self.width / 2) as usize],
            self.v_plane[(x + y * self.width / 2) as usize],
        )
    }

    pub fn uv_width(&self) -> u32 {
        self.width / 2
    }

    pub fn uv_height(&self) -> u32 {
        self.height / 2
    }
}
