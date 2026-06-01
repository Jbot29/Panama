use std::{fs::File, io::BufReader, path::Path};

use rodio::{Decoder, OutputStream, OutputStreamHandle, Sink};

pub struct AudioPlayer {
    // Must be kept alive for audio to continue playing.
    _stream: OutputStream,
    handle: OutputStreamHandle,

    // Keep the sink alive if you want playback to continue.
    sink: Option<Sink>,
}

#[allow(dead_code)]
impl AudioPlayer {
    pub fn new() -> anyhow::Result<Self> {
        let (_stream, handle) = OutputStream::try_default()?;
        Ok(Self {
            _stream,
            handle,
            sink: None,
        })
    }

    /// Play an MP3 (or any supported format) from disk.
    /// If something is already playing, it stops it first.
    pub fn play_file<P: AsRef<Path>>(&mut self, path: P) -> anyhow::Result<()> {
        self.stop();

        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let source = Decoder::new(reader)?; // mp3 works with the feature flag
        let sink = Sink::try_new(&self.handle)?;
        sink.append(source);
        sink.play();

        self.sink = Some(sink);
        Ok(())
    }

    pub fn stop(&mut self) {
        if let Some(sink) = &self.sink {
            // empties queue; stops playback
            sink.stop();
        }
        self.sink = None;
    }

    pub fn set_volume(&mut self, volume_0_to_1: f32) {
        if let Some(sink) = &self.sink {
            sink.set_volume(volume_0_to_1.clamp(0.0, 1.0));
        }
    }

    pub fn is_playing(&self) -> bool {
        self.sink.as_ref().is_some_and(|s| !s.empty())
    }
}
