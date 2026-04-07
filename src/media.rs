use std::{
    fmt::{self, Display},
    hash::{DefaultHasher, Hash, Hasher},
};

use iced::widget;
use smol::{io, process};

use ffmpeg_next as ffmpeg;

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Preview {
    pub seek: f64,
    pub input: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PreviewError {
    Raw(ffmpeg::Error),
    SameHash,
    NoPackets,
}

impl Display for PreviewError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PreviewError::Raw(e) => e.fmt(f),
            PreviewError::SameHash => write!(f, "same encoded hash"),
            PreviewError::NoPackets => write!(f, "no valid packets in input"),
        }
    }
}

impl std::error::Error for PreviewError {}

impl Preview {
    pub async fn decode_image(
        self,
        prev_hash: u64,
    ) -> Result<(widget::image::Handle, u64), PreviewError> {
        let mut ictx = ffmpeg::format::input(&self.input).map_err(PreviewError::Raw)?;

        let input = ictx
            .streams()
            .best(ffmpeg_next::media::Type::Video)
            .ok_or(ffmpeg::Error::StreamNotFound)
            .map_err(PreviewError::Raw)?;

        let context_decoder = ffmpeg::codec::context::Context::from_parameters(input.parameters())
            .map_err(PreviewError::Raw)?;

        let mut decoder = context_decoder
            .decoder()
            .video()
            .map_err(PreviewError::Raw)?;

        let mut scalar = ffmpeg::software::scaling::Context::get(
            decoder.format(),
            decoder.width(),
            decoder.height(),
            ffmpeg::format::Pixel::RGB24,
            decoder.width(),
            decoder.height(),
            ffmpeg::software::scaling::Flags::BILINEAR,
        )
        .map_err(PreviewError::Raw)?;

        let target_stream = input.index();
        let mut decoded = ffmpeg::util::frame::video::Video::empty();
        let mut rgb_frame = ffmpeg::util::frame::video::Video::empty();

        ictx.seek((self.seek * 1_000_000.0).round() as i64, i64::MIN..i64::MAX)
            .map_err(PreviewError::Raw)?;

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
                return Err(PreviewError::SameHash);
            }

            decoder.send_packet(&packet).map_err(PreviewError::Raw)?;

            match decoder.receive_frame(&mut decoded) {
                // skip the rest of the loop on benign "Resource temporarily unavailable" error
                Err(ffmpeg::Error::Other { errno: 11 }) => continue,
                Err(e) => return Err(PreviewError::Raw(e)),
                _ => {}
            }

            scalar
                .run(&decoded, &mut rgb_frame)
                .map_err(PreviewError::Raw)?;

            let mut buf = Vec::new();
            for (i, rgb) in rgb_frame.data(0).iter().enumerate() {
                buf.push(*rgb);
                if (i + 1) % 3 == 0 {
                    buf.push(u8::MAX);
                }
            }

            let handle =
                widget::image::Handle::from_rgba(rgb_frame.width(), rgb_frame.height(), buf);

            return Ok((handle, new_hash));
        }

        Err(PreviewError::NoPackets)
    }
}

/// A handle over mpv
#[derive(Debug, Default)]
pub struct Player(Option<process::Child>);
impl Player {
    /// Spawns a new player,
    /// discarding the handle to the previous one
    pub fn play(
        &mut self,
        preview: Preview,
        secs: isize,
        video: bool,
        audio: bool,
    ) -> io::Result<()> {
        let start_arg = format!("--start={}", preview.seek);
        let length_arg = format!("--length={}", secs);

        let mut args = vec![
            &start_arg,
            &length_arg,
            "--no-config",
            "--volume=70",
            "--terminal=no",
            &preview.input,
        ];

        if !video {
            args.push("--video=no")
        } else {
            args.push("--player-operation-mode=pseudo-gui")
        }
        if !audio {
            args.push("--audio=no");
        }

        self.0 = Some(process::Command::new("mpv").args(args).spawn()?);

        Ok(())
    }

    fn child_is_active(&mut self) -> bool {
        match self.0 {
            Some(ref mut child) => match &child.try_status() {
                Ok(opt) => match opt {
                    Some(status) => {
                        if !status.success() {
                            eprintln!("player failed with status: {status}");
                        }
                        false
                    }
                    None => true,
                },
                Err(e) => {
                    eprintln!("failed to check status of player: {e}");
                    false
                }
            },
            None => false,
        }
    }

    fn is_active(&mut self) -> bool {
        let is_active = self.child_is_active();
        if !is_active {
            self.0 = None;
        }

        is_active
    }

    fn kill(&mut self) -> io::Result<()> {
        if let Some(ref mut child) = self.0 {
            child.kill()
        } else {
            Ok(())
        }
    }

    fn toggle(&mut self, preview: Preview, secs: isize, video: bool, audio: bool) {
        if self.is_active() {
            #[allow(unused_must_use)]
            self.kill()
                .inspect_err(|e| eprintln!("failed to kill player: {e}"));
        } else {
            #[allow(unused_must_use)]
            self.play(preview, secs, video, audio)
                .inspect_err(|e| eprintln!("failed to play preview: {e}"));
        }
    }

    pub fn toggle_preview(&mut self, media: &Media, seek: f64) {
        self.toggle(
            Preview {
                seek,
                input: media.input.clone(),
            },
            5,
            media.use_video,
            media.use_audio,
        );
    }
}

#[derive(Debug, Default, PartialEq, Clone)]
pub struct Media {
    pub start: f64,
    pub end: f64,

    pub input: String,
    pub output: String,

    pub use_video: bool,
    pub use_audio: bool,
    pub use_subs: bool,
    pub use_extra_streams: bool,
}

impl Media {
    /// uses the parameters and the input to create the output
    pub async fn create(self) -> Result<(), String> {
        let seek = self.start.to_string();
        let end = self.end.to_string();

        #[rustfmt::skip]
        let mut args = vec![
            "-ss",  &seek,
            "-to",  &end,
            "-i",   &self.input,
        ];

        if self.use_audio {
            args.push("-c:a");
            args.push("copy");
        } else {
            args.push("-an");
        }

        if self.use_video {
            args.push("-c:v");
            args.push("copy");
        } else {
            args.push("-vn");
        }

        if self.use_subs {
            args.push("-c:s");
            args.push("copy");
        } else {
            args.push("-sn");
        }

        if self.use_extra_streams {
            args.push("-map");
            args.push("0");
        }

        args.push(&self.output);

        match process::Command::new("ffmpeg").args(&args).spawn() {
            Err(e) => Err(e.to_string()),
            Ok(mut child) => match child.status().await {
                Err(e) => Err(e.to_string()),
                Ok(status) => {
                    if status.success() {
                        Ok(())
                    } else {
                        Err(format!(
                            "ffmpeg returned {status}. Check stderr for full error"
                        ))
                    }
                }
            },
        }
    }

    /// updates the Media with the input parameters, returning the input length.
    /// by default, we use all streams that exist
    pub fn update_video_params(&mut self) -> Result<f64, ffmpeg::Error> {
        // try to load the media
        let context = ffmpeg::format::input(&self.input)?;

        let mut streams = context.streams();

        self.use_video =
            streams.any(|stream| stream.parameters().medium() == ffmpeg::media::Type::Video);

        self.use_audio =
            streams.any(|stream| stream.parameters().medium() == ffmpeg::media::Type::Audio);

        self.use_subs =
            streams.any(|stream| stream.parameters().medium() == ffmpeg::media::Type::Subtitle);

        self.use_extra_streams = context.nb_streams()
            > self.use_video as u32 + self.use_audio as u32 + self.use_subs as u32;

        Ok(context.duration() as f64 / f64::from(ffmpeg::ffi::AV_TIME_BASE))
    }
}
