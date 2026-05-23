use ffmpeg_next as ffmpeg;

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
    pub async fn create(self) -> Result<(), ffmpeg::Error> {
        let mut ictx = ffmpeg::format::input(&self.input)?;
        let mut octx = ffmpeg::format::output(&self.output)?;

        ictx.seek(
            (self.start * 1_000_000.0).round() as i64,
            i64::MIN..i64::MAX,
        )?;

        let mut stream_mapping = vec![0; ictx.nb_streams() as _];
        let mut ist_time_bases = vec![ffmpeg::Rational(0, 1); ictx.nb_streams() as _];
        let mut ost_index = 0;
        for (ist_index, ist) in ictx.streams().enumerate() {
            let ist_medium = ist.parameters().medium();
            if !{
                use ffmpeg::media::Type;
                match ist_medium {
                    Type::Video => self.use_video,
                    Type::Audio => self.use_audio,
                    Type::Subtitle => self.use_subs,
                    _ => self.use_extra_streams,
                }
            } {
                stream_mapping[ist_index] = -1;
                continue;
            }
            stream_mapping[ist_index] = ost_index;
            ist_time_bases[ist_index] = ist.time_base();
            ost_index += 1;
            let mut ost = octx.add_stream(ffmpeg::encoder::find(ffmpeg::codec::Id::None))?;
            ost.set_parameters(ist.parameters());
            // We need to set codec_tag to 0 lest we run into incompatible codec tag
            // issues when muxing into a different container format. Unfortunately
            // there's no high level API to do this (yet).
            unsafe {
                (*ost.parameters().as_mut_ptr()).codec_tag = 0;
            }
        }

        octx.set_metadata(ictx.metadata().to_owned());
        octx.write_header()?;

        for (stream, mut packet) in ictx.packets() {
            assert_ne!(stream.time_base().numerator(), 0);

            if packet
                .pts()
                .expect("packet should contain a Presentation TimeStamp")
                >= (self.end / f64::from(stream.time_base())).round() as i64
            {
                continue;
            }

            let ist_index = stream.index();
            let ost_index = stream_mapping[ist_index];
            if ost_index < 0 {
                continue;
            }
            let ost = octx
                .stream(ost_index as _)
                .expect("there should always be an output stream at this index");
            packet.rescale_ts(ist_time_bases[ist_index], ost.time_base());
            packet.set_position(-1);
            packet.set_stream(ost_index as _);
            packet.write_interleaved(&mut octx)?;
        }

        octx.write_trailer()?;

        Ok(())
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
