use super::*;

use smol::block_on;
use std::{env, fs};

macro_rules! TESTFILE {
    ($v:literal) => {
        env::var(format!("TESTFILE{}", $v)).expect("test file should be avaliable")
    };
}

#[test]
fn test_create() {
    let media = Media {
        start: 60.0,
        end: 120.0,
        input: TESTFILE!(0),
        output: String::from("test.mkv"),
        use_video: true,
        use_audio: true,
        use_subs: true,
        use_extra_streams: true,
    };

    block_on(media.clone().create()).unwrap();

    // since the hash is mismatched each time, but the size isn't, why not
    let new_size = fs::metadata(&media.output).unwrap().len();
    assert_eq!(new_size, 3480879, "size test");

    let context = ffmpeg::format::input(&media.output).unwrap();
    assert_eq!(context.duration(), 60575000, "duration test");
}

#[test]
fn test_decode_image() {
    let preview = preview::Preview {
        seek: 60.0,
        input: TESTFILE!(0),
    };

    assert_eq!(
        block_on(preview.decode_image(13300275450624329887)),
        Err(preview::Error::SameHash),
        "packet hash test"
    );
}
