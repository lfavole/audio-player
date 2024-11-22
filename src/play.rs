use rodio::{Decoder, OutputStream, Sink, Source};
use rand::prelude::*;
use crate::song::{check_double_songs, EBox, Song};

pub async fn play_songs<'name, T: Song<'name>>(songs: &mut [T]) -> Result<(), EBox> {
    let mut rng = thread_rng();
    let (_stream, stream_handle) = OutputStream::try_default()?;
    let sink = Sink::try_new(&stream_handle)?;

    let queue = &mut songs[..];
    let length = queue.len();
    let mut position = 0;

    loop {
        if position == 0 {
            queue.shuffle(&mut rng);
            check_double_songs(queue);
        }
        let song = &mut queue[position];
        println!("{}", song.get_path());

        let source = Decoder::new_mp3(song.get_data().await?)?.buffered();
        sink.append(source);

        if position + 1 != length {
            queue[position + 1].preload().await?;
        }

        sink.sleep_until_end();
        if position == length {
            position = 0;
        } else {
            position += 1;
        }
    }
}
