use ffmpeg_next as ffmpeg;

use smol::process;

pub mod player;
pub mod preview;

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
            "-loglevel", "warning",
            "-y",
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

    /// this function should make sure that the start and end values are reasonable,
    /// regardless of when it is called.
    ///
    /// It is however a little disruptive to user input;
    /// call this function when user input has ceased.
    pub fn clamp_numbers(&mut self, input_length: f64) {
        if self.start < 0.0 {
            self.start = 0.0;
        }
        if self.end < 0.0 {
            self.end = 0.0;
        }

        if self.end > input_length {
            self.end = input_length;
        }

        if self.start > self.end {
            self.start = self.end;
        }

        if self.end < self.start {
            self.end = self.start;
        }
    }
}
