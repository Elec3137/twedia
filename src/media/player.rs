use smol::{io, process};

use super::Media;

/// A handle over mpv
#[derive(Debug, Default)]
pub struct Player(Option<process::Child>);
impl Player {
    /// Spawns a new player,
    /// discarding the handle to the previous one
    pub fn play(
        &mut self,
        path: &str,
        start: usize,
        length: usize,
        video: bool,
        audio: bool,
        subs: bool,
    ) -> io::Result<()> {
        let start_arg = format!("--start={}", start);
        let length_arg = format!("--length={}", length);

        let mut args = vec![
            &start_arg,
            &length_arg,
            "--no-config",
            "--volume=70",
            "--player-operation-mode=pseudo-gui",
            "--keep-open",
            path,
        ];

        if !video {
            args.push("--video=no")
        }
        if !audio {
            args.push("--audio=no");
        }
        if !subs {
            args.push("--sub=no");
        }

        self.0 = Some(process::Command::new("mpv").args(args).spawn()?);

        Ok(())
    }

    fn is_active(&mut self) -> bool {
        let is_active = match self.0 {
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
        };

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

    fn toggle(
        &mut self,
        path: &str,
        start: usize,
        length: usize,
        video: bool,
        audio: bool,
        subs: bool,
    ) {
        if self.is_active() {
            #[allow(unused_must_use)]
            self.kill()
                .inspect_err(|e| eprintln!("failed to kill player: {e}"));
        } else {
            #[allow(unused_must_use)]
            self.play(path, start, length, video, audio, subs)
                .inspect_err(|e| eprintln!("failed to play preview: {e}"));
        }
    }

    /// Higher level function that calls `Self::toggle`
    /// using the fields of the `Media` type.
    ///
    /// Note: does not use `Media::output` or `Media::use_extra_streams`
    pub fn toggle_preview_of(&mut self, media: &Media) {
        self.toggle(
            &media.input,
            media.start.round() as _,
            (media.end - media.start).round() as _,
            media.use_video,
            media.use_audio,
            media.use_subs,
        );
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        #[allow(unused_must_use)]
        self.kill();
    }
}
