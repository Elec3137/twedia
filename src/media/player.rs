use std::{
    env::{self},
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
};

use super::Media;

#[derive(Debug, Default)]
pub struct Player {
    last_tmp: Option<PathBuf>,
}
impl Player {
    pub fn play(&mut self, mut media: Media) -> Result<(), Error> {
        let tmp = env::temp_dir()
            .join(
                Path::new(&media.output)
                    .file_name()
                    .unwrap_or(OsStr::new("media.mkv")),
            )
            .with_extension("mp4");
        media.output = tmp.to_string_lossy().to_string();

        if let Some(path) = self.last_tmp.take() {
            fs::remove_file(path)?;
        }
        media.create()?;

        open::that(&tmp)?;
        self.last_tmp = Some(tmp);
        Ok(())
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        if let Some(path) = self.last_tmp.take()
            && let Err(e) = std::fs::remove_file(path)
        {
            eprintln!("failed to remove Player.last_tmp: {e}")
        }
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    FFmpeg(#[from] ffmpeg_next::Error),
}
