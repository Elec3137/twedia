use std::{
    fmt::{self, Display},
    hash::{DefaultHasher, Hash, Hasher},
};

use iced::{Size, widget};

use ffmpeg_next as ffmpeg;

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

impl Preview {
    pub async fn decode_image(
        self,
        prev_hash: u64,
    ) -> Result<(widget::image::Handle, u64, Size), Error> {
        let mut ictx = ffmpeg::format::input(&self.input)?;

        let input = ictx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)?;

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())?;

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

        let target_stream = input.index();
        let mut decoded = ffmpeg::util::frame::video::Video::empty();
        let mut rgba_frame = ffmpeg::util::frame::video::Video::empty();

        // 1_000_000 is to convert seconds to ffmpeg's time scale;
        // likely equivelent to `ffmpeg::ffi::AV_TIME_BASE`
        ictx.seek((self.seek * 1_000_000.0).round() as i64, i64::MIN..i64::MAX)?;

        for packet in ictx.packets().filter_map(|(stream, packet)| {
            if stream.index() == target_stream {
                Some(packet)
            } else {
                None
            }
        }) {
            // skip empty packets
            if unsafe { packet.is_empty() } {
                continue;
            }

            let mut hasher = DefaultHasher::new();
            packet.data().hash(&mut hasher);
            let new_hash = hasher.finish();

            // make sure that the hash is different before decoding
            if new_hash == prev_hash {
                return Err(Error::SameHash);
            }

            decoder.send_packet(&packet)?;

            match decoder.receive_frame(&mut decoded) {
                // skip the rest of the loop on benign "Resource temporarily unavailable" error
                Err(ffmpeg::Error::Other { errno: 11 }) => continue,
                Err(e) => return Err(Error::Raw(e)),
                _ => {}
            }

            scalar.run(&decoded, &mut rgba_frame)?;

            let handle = widget::image::Handle::from_rgba(
                rgba_frame.width(),
                rgba_frame.height(),
                rgba_frame.data(0).to_owned(),
            );

            return Ok((
                handle,
                new_hash,
                Size::new(rgba_frame.width() as f32, rgba_frame.height() as f32),
            ));
        }

        Err(Error::NoPackets)
    }
}
