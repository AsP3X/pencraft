//! The welcome film: what a new user sees the first time Quill starts. In under ten seconds it shows the three
//! things the app is for (describe a design, watch an assistant draw it, change it yourself), then the name
//! and two buttons.
//!
//! It has sound: music, with effects on its cuts ([`sound`]). A computer without speakers plays it silently.
//!
//! It is a film in the plain sense: every frame follows from the seconds played alone, so it can be skipped
//! to its end, and posed at any moment for a picture. `design/onboarding/splash.html` is the same film in a
//! browser, where it was designed; the numbers here are that page's.

mod draw;
mod scenes;
mod sound;

use std::time::Instant;

use egui::{Color32, CursorIcon, Key, Rect, Sense, TextureHandle, TextureOptions, Ui};

use crate::theme::{SPLASH_INK, SPLASH_LENGTH, SPLASH_PAPER, SPLASH_READY, SPLASH_STAGE};
use draw::{Pen, View};

/// The pictures of the film, in the order [`Pictures`] keeps them: the stack of books, the four covers, the
/// fourteen designs on the wall, the finished page.
static FILES: [&[u8]; 20] = [
    include_bytes!("../assets/splash/hero.png"),
    include_bytes!("../assets/splash/cover1.jpg"),
    include_bytes!("../assets/splash/cover2.jpg"),
    include_bytes!("../assets/splash/cover3.jpg"),
    include_bytes!("../assets/splash/cover4.jpg"),
    include_bytes!("../assets/splash/wall01.jpg"),
    include_bytes!("../assets/splash/wall02.jpg"),
    include_bytes!("../assets/splash/wall03.jpg"),
    include_bytes!("../assets/splash/wall04.jpg"),
    include_bytes!("../assets/splash/wall05.jpg"),
    include_bytes!("../assets/splash/wall06.jpg"),
    include_bytes!("../assets/splash/wall07.jpg"),
    include_bytes!("../assets/splash/wall08.jpg"),
    include_bytes!("../assets/splash/wall09.jpg"),
    include_bytes!("../assets/splash/wall10.jpg"),
    include_bytes!("../assets/splash/wall11.jpg"),
    include_bytes!("../assets/splash/wall12.jpg"),
    include_bytes!("../assets/splash/wall13.jpg"),
    include_bytes!("../assets/splash/wall14.jpg"),
    include_bytes!("../assets/splash/board.jpg"),
];
/// Where the stack of books is among the pictures.
const HERO: usize = 0;
/// Where the first of the four covers is.
const COVER: usize = 1;
/// Where the first of the wall's fourteen designs is.
const WALL: usize = 5;
/// Where the finished page is.
const BOARD: usize = 19;

/// The film's pictures as the graphics card holds them; one that couldn't be read is left out and its place
/// is drawn without it.
pub struct Pictures(Vec<Option<TextureHandle>>);

impl Pictures {
    /// Reads every picture.
    fn read(ctx: &egui::Context) -> Pictures {
        let read = |(index, bytes): (usize, &&[u8])| {
            let picture = match image::load_from_memory(bytes) {
                Ok(picture) => picture.to_rgba8(),
                Err(problem) => {
                    log::warn!("welcome film: picture {index} is left out: {problem}");
                    return None;
                }
            };
            let size = [usize::try_from(picture.width()).ok()?, usize::try_from(picture.height()).ok()?];
            let pixels = egui::ColorImage::from_rgba_unmultiplied(size, picture.as_raw());
            Some(ctx.load_texture(format!("welcome {index}"), pixels, TextureOptions::LINEAR))
        };
        Pictures(FILES.iter().enumerate().map(read).collect())
    }

    /// The picture at `index`, when it could be read.
    fn get(&self, index: usize) -> Option<&TextureHandle> {
        self.0.get(index).and_then(Option::as_ref)
    }
}

/// What the user chose at the end of the film.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Nothing yet: the film is playing, or waiting at its end.
    Playing,
    /// Go on to the app.
    Start,
    /// Go on to the app with the settings open where assistants are set up.
    Assistant,
}

/// The welcome film while it shows.
pub struct Splash {
    began: Instant,
    /// A moment the film stands still at (for a picture of it), in seconds.
    posed: Option<f32>,
    pictures: Option<Pictures>,
    /// The film's sound while it plays; `None` for a film that stands still, is silent, or was skipped.
    sound: Option<sound::Sound>,
}

impl Splash {
    /// A film that starts now, or one standing still at `posed` seconds.
    #[must_use]
    pub fn new(posed: Option<f32>) -> Splash {
        // Human: the page's headline is set in two faces of the computer's own; asking for them now has them
        // ready by the time the page is drawn.
        crate::fonts::want(draw::SERIF, 700, false);
        crate::fonts::want(draw::HEAVY, 900, false);
        Splash { began: Instant::now(), posed, pictures: None, sound: None }
    }

    /// Draws the film's frame of this moment over the whole of `ui`, and says what the user chose.
    // Human: with animations turned off in the settings there is no film: its last frame shows at once.
    pub fn show(&mut self, ui: &mut Ui, animations: bool) -> Outcome {
        let ctx = ui.ctx().clone();
        if self.pictures.is_none() {
            // Human: the film starts with its first frame, once its pictures are read, and its sound with it, so
            // the two run together. Only a film that moves has sound.
            self.pictures = Some(Pictures::read(&ctx));
            self.began = Instant::now();
            if self.posed.is_none() && animations {
                self.sound = sound::Sound::start();
            }
        }
        if self.sound.as_ref().is_some_and(sound::Sound::over) {
            self.sound = None;
        }
        let now = Instant::now();
        let played = now.saturating_duration_since(self.began).as_secs_f32();
        let mut time = self.posed.unwrap_or(if animations { played } else { SPLASH_LENGTH }).min(SPLASH_LENGTH);
        let window = ui.max_rect();
        let sense = ui.interact(window, egui::Id::unique("welcome film"), Sense::click());
        let escape = ctx.input(|input| input.key_pressed(Key::Escape));
        if time < SPLASH_READY && self.posed.is_none() && (sense.clicked() || escape) {
            // Human: a click or Esc during the film goes straight to its end, where the buttons are, and ends
            // its sound, which would no longer fit the picture.
            self.sound = None;
            self.began = now.checked_sub(std::time::Duration::from_secs_f32(SPLASH_READY)).unwrap_or(now);
            time = SPLASH_READY;
        } else if escape {
            return Outcome::Start;
        }
        let fit = (window.width() / SPLASH_STAGE.x).min(window.height() / SPLASH_STAGE.y);
        let origin = window.center() - SPLASH_STAGE * fit / 2.0;
        let pen = Pen::new(ui.painter().with_clip_rect(window), View { origin, scale: fit });
        // Human: the stage keeps its shape; what the window has more of is filled with the beat's ground.
        ui.painter().rect_filled(window, 0.0, ground_at(time));
        if let Some(pictures) = &self.pictures {
            scenes::frame(&pen, pictures, time);
        }
        if time < SPLASH_LENGTH && self.posed.is_none() {
            ctx.request_repaint();
        }
        if time < SPLASH_READY {
            return Outcome::Playing;
        }
        buttons(ui, &pen)
    }
}

/// The colour of the film's ground at `time`.
fn ground_at(time: f32) -> Color32 {
    if (scenes::WATCH..scenes::YOURS).contains(&time) || time >= scenes::NAME { SPLASH_PAPER } else { SPLASH_INK }
}

/// The two buttons at the end, which the last scene has drawn: this is where they are pressed.
fn buttons(ui: &Ui, pen: &Pen) -> Outcome {
    let [start, assistant] = scenes::BUTTONS.map(|[x, y, w, h]| pen.view.rect(x, y, w, h));
    let press = |rect: Rect, name: &'static str| {
        let button = ui.interact(rect, egui::Id::unique(name), Sense::click());
        if button.hovered() {
            ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
        }
        button.clicked()
    };
    if press(start, "welcome start") || ui.input(|input| input.key_pressed(Key::Enter)) {
        Outcome::Start
    } else if press(assistant, "welcome assistant") {
        Outcome::Assistant
    } else {
        Outcome::Playing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ground_is_dark_then_light_then_dark_and_light_under_the_name() {
        assert_eq!(ground_at(0.5), SPLASH_INK);
        assert_eq!(ground_at(3.0), SPLASH_PAPER);
        assert_eq!(ground_at(5.0), SPLASH_INK);
        assert_eq!(ground_at(7.0), SPLASH_INK);
        assert_eq!(ground_at(9.0), SPLASH_PAPER);
    }

    #[test]
    fn every_picture_of_the_film_can_be_read() {
        for (index, bytes) in FILES.iter().enumerate() {
            assert!(image::load_from_memory(bytes).is_ok(), "picture {index}");
        }
    }
}
