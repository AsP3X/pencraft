//! The welcome film's sound: one track of music with the effects mixed in at the film's own moments
//! (`design/onboarding` says how it was made), played from its start when the film starts.

use std::io::Cursor;

/// The track: eleven seconds, so the music fades out a little after the film has come to rest.
static TRACK: &[u8] = include_bytes!("../../assets/splash/soundtrack.mp3");
/// How loud it is played, as a share of the file's own loudness.
const VOLUME: f32 = 0.8;

/// The track while it plays. Dropping it stops the sound.
pub struct Sound {
    player: rodio::Player,
    // Human: the sound goes out through this for as long as it is kept; it must outlive the player.
    _speakers: rodio::MixerDeviceSink,
}

impl Sound {
    /// Starts the track on the computer's speakers. `None` when there are no speakers to play on, or the track
    /// can't be read: the film then plays silently.
    #[must_use]
    pub fn start() -> Option<Sound> {
        let mut speakers = match rodio::DeviceSinkBuilder::open_default_sink() {
            Ok(speakers) => speakers,
            Err(problem) => {
                log::warn!("welcome film: it plays without sound: {problem}");
                return None;
            }
        };
        // Agent: the library would otherwise write a line to the terminal when the speakers are let go.
        speakers.log_on_drop(false);
        match rodio::play(speakers.mixer(), Cursor::new(TRACK)) {
            Ok(player) => {
                player.set_volume(VOLUME);
                Some(Sound { player, _speakers: speakers })
            }
            Err(problem) => {
                log::warn!("welcome film: it plays without sound: {problem}");
                None
            }
        }
    }

    /// Whether the track has played to its end.
    #[must_use]
    pub fn over(&self) -> bool {
        self.player.empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_track_is_an_mp3_file() {
        // Human: an MP3 file starts with an ID3 tag or with a frame's sync bits.
        let tagged = TRACK.starts_with(b"ID3");
        let framed = TRACK.first() == Some(&0xff) && TRACK.get(1).is_some_and(|byte| byte & 0xe0 == 0xe0);
        assert!(tagged || framed);
        assert!(TRACK.len() > 100_000);
    }
}
