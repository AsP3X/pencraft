//! `--screenshot <file.png>`: draw about 120 frames, save a picture of the window, quit.
//!
//! Run this way after a build to see what the window looks like.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use crate::app::{ShotSlot, lock};

/// Frames drawn before the picture is taken (fonts, layout and the first zoom-to-fit have settled by then).
const FRAMES_BEFORE_SHOT: u64 = 120;

/// Where the window is put while the picture is made: far outside any screen.
const OFF_SCREEN: [f32; 2] = [-30000.0, -30000.0];

/// One pending screenshot.
pub struct Job {
    path: PathBuf,
    frames: u64,
    requested: bool,
    slot: ShotSlot,
}

impl Job {
    /// A screenshot to be saved at `path`.
    #[must_use]
    pub fn new(path: PathBuf) -> Job {
        Job { path, frames: 0, requested: false, slot: Arc::new(Mutex::new(None)) }
    }

    /// Called every frame: keeps frames coming, asks for the picture, saves it and quits.
    pub fn frame(&mut self, ctx: &egui::Context) {
        ctx.request_repaint();
        // Human: macOS puts a new window on a screen wherever it was asked for; moved afterwards it stays away.
        if self.frames == 0 {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(OFF_SCREEN[0], OFF_SCREEN[1])));
        }
        self.frames += 1;
        if !self.requested && self.frames >= FRAMES_BEFORE_SHOT {
            self.requested = true;
            log::info!("screenshot: the window is at {:?}", ctx.input(|i| i.viewport().outer_rect));
            let slot = self.slot.clone();
            let wake = ctx.clone();
            ctx.request_screenshot(move |image| {
                *lock(&slot) = Some(image);
                wake.request_repaint();
            });
        }
        let image = lock(&self.slot).take();
        if let Some(image) = image {
            let code = match write_png(&self.path, &image) {
                Ok(()) => 0,
                Err(e) => {
                    log::error!("screenshot: {}: {e}", self.path.display());
                    1
                }
            };
            // Human: the picture is the whole job; leave at once instead of going through the quit path.
            std::process::exit(code);
        }
    }
}

/// Writes an egui picture as an 8-bit RGBA PNG.
fn write_png(path: &std::path::Path, image: &egui::ColorImage) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
        std::fs::create_dir_all(dir)?;
    }
    let [w, h] = image.size;
    let mut bytes = Vec::with_capacity(w * h * 4);
    for pixel in &image.pixels {
        bytes.extend_from_slice(&pixel.to_srgba_unmultiplied());
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, u32::try_from(w)?, u32::try_from(h)?);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header()?;
        writer.write_image_data(&bytes)?;
    }
    craft_atomic::atomic_write(path, &out)?;
    Ok(())
}
