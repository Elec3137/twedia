use super::*;

use smol::block_on;
use std::{env, fs};

#[test]
fn test_create() {
    let media = Media {
        start: 60.0,
        end: 120.0,
        input: env::var("TESTFILE0").expect("test file should be avaliable"),
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
