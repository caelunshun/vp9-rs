use std::{io::Read, iter};

use byteorder::{LittleEndian, ReadBytesExt};

#[derive(Debug, thiserror::Error)]
pub enum IvfError {
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error("file does not begin with IVF signature")]
    MissingSignature,
    #[error("file uses unsupported codec '{0}'")]
    UnsupportedCodec(String),
    #[error("frame is too large")]
    FrameTooLarge,
}

pub struct IvfDemuxer<R> {
    reader: R,
    header: IvfHeader,
    current_frame: u32,
    frame_buffer: Vec<u8>,
}

impl<R> IvfDemuxer<R>
where
    R: Read,
{
    pub fn new(mut reader: R) -> Result<Self, IvfError> {
        // Read the header.
        let mut signature = [0u8; 4];
        reader.read_exact(&mut signature)?;
        if signature != *b"DKIF" {
            return Err(IvfError::MissingSignature);
        }

        let _version = reader.read_u16::<LittleEndian>()?;
        let _header_length = reader.read_u16::<LittleEndian>()?;

        let mut codec = [0u8; 4];
        reader.read_exact(&mut codec)?;
        if codec != *b"VP90" {
            return Err(IvfError::UnsupportedCodec(
                std::str::from_utf8(&codec).unwrap_or_default().to_owned(),
            ));
        }

        let width = reader.read_u16::<LittleEndian>()? as u32;
        let height = reader.read_u16::<LittleEndian>()? as u32;

        let time_base_denom = reader.read_u32::<LittleEndian>()?;
        let time_base_num = reader.read_u32::<LittleEndian>()?;

        let number_of_frames = reader.read_u32::<LittleEndian>()?;

        let _unused = reader.read_u32::<LittleEndian>()?;

        let header = IvfHeader {
            width,
            height,
            time_base_denom,
            time_base_num,
            number_of_frames,
        };

        Ok(Self {
            reader,
            header,
            current_frame: 0,
            frame_buffer: Vec::new(),
        })
    }

    pub fn next_frame(&mut self) -> Result<Option<IvfFrame>, IvfError> {
        if self.current_frame >= self.header.number_of_frames {
            return Ok(None);
        }

        let frame_size = self.reader.read_u32::<LittleEndian>()?;
        let timestamp = self.reader.read_u64::<LittleEndian>()?;

        if frame_size > 1024 * 1024 * 8 {
            return Err(IvfError::FrameTooLarge);
        }

        self.frame_buffer.clear();
        self.frame_buffer
            .extend(iter::repeat(0).take(frame_size as usize));
        self.reader.read_exact(&mut self.frame_buffer)?;

        self.current_frame += 1;

        Ok(Some(IvfFrame {
            timestamp,
            data: &self.frame_buffer,
        }))
    }

    pub fn header(&self) -> &IvfHeader {
        &self.header
    }
}

pub struct IvfFrame<'a> {
    pub timestamp: u64,
    pub data: &'a [u8],
}

#[derive(Debug)]
pub struct IvfHeader {
    pub width: u32,
    pub height: u32,
    pub time_base_num: u32,
    pub time_base_denom: u32,
    pub number_of_frames: u32,
}
