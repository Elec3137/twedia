use std::fmt::{self, Display};

use ffmpeg_next as ffmpeg;

use crate::utils;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Preview {
    pub seek: f64,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Error {
    Raw(ffmpeg::Error),
    SameHash,
    NoPackets,
}

impl Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Raw(e) => e.fmt(f),
            Error::SameHash => write!(f, "same encoded hash"),
            Error::NoPackets => write!(f, "no valid packets in input"),
        }
    }
}
impl From<ffmpeg::Error> for Error {
    fn from(value: ffmpeg::Error) -> Self {
        Error::Raw(value)
    }
}
impl std::error::Error for Error {}

/// (width, height),
/// pixel data,
/// hash of the source packet
#[derive(Debug, PartialEq, Clone)]
pub struct Output {
    pub size: (u32, u32),
    pub rgba: Vec<u8>,
    pub hash: u64,
}
impl From<Output> for iced::widget::image::Handle {
    fn from(value: Output) -> Self {
        iced::widget::image::Handle::from_rgba(value.size.0, value.size.1, value.rgba)
    }
}

impl Preview {
    pub async fn decode_image(self, prev_hash: u64) -> Result<Output, Error> {
        let mut context = ffmpeg::format::input(&self.input)?;

        let input_stream = context
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;

        let context_decoder =
            ffmpeg::codec::context::Context::from_parameters(input_stream.parameters())?;

        let mut decoder = context_decoder.decoder().video()?;

        let mut scalar = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGBA,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )?;

        let target_stream = input_stream.index();
        let mut decoded = ffmpeg::util::frame::video::Video::empty();
        let mut rgba_frame = ffmpeg::util::frame::video::Video::empty();

        context.seek(
            (self.seek * f64::from(ffmpeg::ffi::AV_TIME_BASE)).round() as i64,
            i64::MIN..i64::MAX,
        )?;

        for packet in context.packets().filter_map(|(stream, packet)| {
            if stream.index() == target_stream {
                Some(packet)
            } else {
                None
            }
        }) {
            // skip empty packets
            if unsafe { packet.is_empty() } {
                eprintln!("packet {:?} is empty, skipping", packet.pts());
                continue;
            }

            let new_hash = utils::hash_chunk(packet.data().unwrap());

            // make sure that the hash is different before decoding
            if new_hash == prev_hash {
                return Err(Error::SameHash);
            }

            decoder.send_packet(&packet)?;

            match decoder.receive_frame(&mut decoded) {
                // skip the rest of the loop on benign "Resource temporarily unavailable" error
                Err(ffmpeg::Error::Other { errno: 11 }) => continue,
                Err(e) => Err(e)?,
                Ok(()) => {}
            }

            scalar.run(&decoded, &mut rgba_frame)?;

            return Ok(Output {
                size: (rgba_frame.width(), rgba_frame.height()),
                rgba: rgba_frame.data(0).to_owned(),
                hash: new_hash,
            });
        }

        Err(Error::NoPackets)
    }
}
