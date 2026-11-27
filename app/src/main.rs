//! Quill: a canvas design editor for `.quill` and `.pen` files, written in Rust as a Craft app.
//!
//! `pencraft [FILE.quill|FILE.pen]... [--screenshot OUT.png [--dark] [--sidebar VIEW] [--settings PAGE] [--select LAYER] [--points] [--find TEXT] [--attach FILE]... [--actions] [--generate-image] [--activity] [--update-prompt] [--update-required] [--update-collapsed] [--update-download [PERCENT]] [--home] [--welcome [SECONDS]] [--export FILE]]`
//! `pencraft [--background] [--private] [--no-update-check] [--update-download [PERCENT]] [--welcome] [--assistant ID] [--ask MESSAGE] [FILE.quill|FILE.pen]...` opens the window behind what the user is doing.
//! `pencraft mcp` connects an AI client to the running app (Model Context Protocol over standard input and output).

// Human: on Windows a shipped (release) build is a window app: started from the Start menu or the Explorer it
// opens no black terminal window beside its own. `pencraft mcp` still talks through the pipes its client
// gives it. A development build stays a terminal program, so its messages show where it was started.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod activity;
mod advice;
mod agent;
mod app;
mod asking;
mod assistants;
mod bundle;
mod canvas;
mod chat;
mod chips;
mod conversation;
mod craft;
mod editor;
mod effects;
mod export;
mod faces;
mod find;
mod fonts;
mod fuzzy;
mod glass_check;
mod gpu;
mod icons;
mod ideas;
mod imagery;
mod images;
mod keys;
mod link;
mod liquid;
mod material;
mod menus;
mod motion;
mod offers;
mod ops;
mod palette;
mod panels;
mod path_edit;
mod pen;
mod prefs;
mod raster;
mod recover;
mod render;
mod review;
mod samples;
mod screenshot;
mod secrets;
mod settings;
mod shader;
mod sidebar;
mod skills;
mod snap;
mod splash;
mod tasklist;
mod theme;
#[cfg(test)]
mod timing;
mod toast;
mod tool_icons;
mod update;
mod web;
mod wire_edit;

use std::path::PathBuf;
use std::process::ExitCode;

/// Default window size.
const WINDOW_SIZE: [f32; 2] = [1352.0, 878.0];
/// Smallest window.
const MIN_WINDOW_SIZE: [f32; 2] = [560.0, 400.0];

/// How many samples a pixel the window is drawn with (multisample anti-aliasing).
const SAMPLES: u16 = 4;
/// The icon of the window and, on a Mac, of the app in the Dock (`design/icon/build.sh` draws both files).
// Human: a Mac icon is a plate with room and a shadow round it, as the Dock expects; elsewhere the plate fills
// the square. Pencraft.app has the same picture as its own icon; this one is for a Quill started without it.
const ICON: &[u8] = if cfg!(target_os = "macos") { include_bytes!("../assets/icon-macos.png") } else { include_bytes!("../assets/icon.png") };
const USAGE: &str = "usage: pencraft [FILE.quill|FILE.pen]... [--screenshot OUT.png [--dark] [--sidebar VIEW] [--settings PAGE] [--select LAYER] [--points] [--find TEXT] [--attach FILE]... [--actions] [--generate-image] [--activity] [--update-prompt] [--update-required] [--update-collapsed] [--update-download [PERCENT]] [--home] [--welcome [SECONDS]] [--export FILE]]\n       pencraft [--background] [--private] [--no-update-check] [--update-download [PERCENT]] [--welcome] [--assistant ID] [--ask MESSAGE] [FILE.quill|FILE.pen]...\n       pencraft mcp
       pencraft skills [show NAME [FILE] | install [--project DIR | --dir DIR]]";

fn main() -> ExitCode {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    // Human: `pencraft mcp` is not the app but the door to it for an AI client: it passes the client's messages
    // to the running app (and starts it when it isn't running).
    if std::env::args().nth(1).as_deref() == Some("mcp") {
        return link::bridge();
    }
    // Human: `pencraft skills` lists the skills and writes them where AI clients look for skills; no window.
    if std::env::args().nth(1).as_deref() == Some("skills") {
        return skills::command(&std::env::args().skip(2).collect::<Vec<_>>());
    }
    let launch = match parse_args(std::env::args().skip(1)) {
        Ok(launch) => launch,
        Err(message) => {
            complain(&format!("{message}\n{USAGE}"));
            return ExitCode::from(2);
        }
    };
    // A check, or an app an AI client started, must never take focus from what the person is doing.
    // Human: an AI client asks for the app in the background; when a Quill is already running there is
    // nothing to start, and no second window opens.
    if launch.background && launch.ask.is_none() && link::serving() {
        return ExitCode::SUCCESS;
    }
    // Human: files opened from the Explorer or the command line go to the Quill that is already running.
    let own_window = launch.screenshot.is_some() || launch.private || launch.background || launch.ask.is_some();
    if !launch.files.is_empty() && !own_window && link::hand_over(&launch.files) {
        return ExitCode::SUCCESS;
    }
    let shooting = launch.screenshot.is_some() || launch.background;
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("Quill")
        .with_app_id("dev.pencraft.desktop")
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size(MIN_WINDOW_SIZE)
        .with_drag_and_drop(true);
    match eframe::icon_data::from_png_bytes(ICON) {
        Ok(icon) => viewport = viewport.with_icon(icon),
        Err(e) => log::warn!("icon: the window keeps the default one: {e}"),
    }
    if cfg!(target_os = "macos") {
        // Human: the tab strip is the title bar, with the window buttons inset at its start.
        viewport = viewport.with_fullsize_content_view(true).with_titlebar_shown(false).with_title_shown(false);
    }
    if cfg!(windows) {
        // Human: on Windows too the tab strip is the title bar: the system's own bar is left off, and the strip
        // has the three window buttons and moves the window (`app::App::tab_bar`); the window's edges resize it
        // (`app::resize_edges`).
        viewport = viewport.with_decorations(false);
    }
    if launch.screenshot.is_some() {
        // Human: a run that only takes a picture keeps its window under every other window, and moves it to
        // where no screen is once it exists (see `screenshot::Job`), so nothing flashes up in front of the person.
        viewport = viewport.with_window_level(egui::WindowLevel::AlwaysOnBottom);
    }
    if shooting {
        viewport = viewport.with_active(false);
    }
    // Human: the canvas draws shapes, paths and icons as plain triangles, which have hard, stepped edges unless
    // the GPU smooths them: four samples a pixel, the level every GPU has.
    let options = eframe::NativeOptions { viewport, renderer: eframe::Renderer::Wgpu, multisampling: SAMPLES, ..Default::default() };
    #[cfg(target_os = "macos")]
    let options = {
        let mut options = options;
        if shooting {
            use winit::platform::macos::EventLoopBuilderExtMacOS as _;
            let picture_only = launch.screenshot.is_some();
            options.event_loop_builder = Some(Box::new(move |builder| {
                builder.with_activate_ignoring_other_apps(false);
                // Human: a run that only takes a picture shows no Dock icon either.
                if picture_only {
                    builder.with_activation_policy(winit::platform::macos::ActivationPolicy::Accessory);
                }
            }));
        }
        options
    };
    let result = eframe::run_native(
        "Quill",
        options,
        Box::new(move |cc| {
            // winit's own Quit (⌘Q) ends the app without asking about unsaved work; this menu's Quit asks first.
            #[cfg(target_os = "macos")]
            if let Err(e) = craft_menu::mac::install_app_menu(&cc.egui_ctx, "Quill") {
                log::warn!("app menu: kept winit's Quit, which ends the app without asking: {e}");
            }
            Ok(Box::new(app::App::new(cc, launch)))
        }),
    );
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            complain(&format!("Quill couldn't open its window: {e}"));
            ExitCode::FAILURE
        }
    }
}

fn parse_args(args: impl Iterator<Item = String>) -> Result<app::Launch, String> {
    let mut launch = app::Launch {
        files: Vec::new(),
        screenshot: None,
        dark: false,
        sidebar: None,
        settings: None,
        select: None,
        actions: false,
        generate_image: false,
        export: None,
        background: false,
        activity: false,
        points: false,
        find: None,
        attach: Vec::new(),
        ask: None,
        assistant: None,
        private: false,
        no_update_check: false,
        update_prompt: false,
        update_download: None,
        update_required: false,
        update_collapsed: false,
        home: false,
        welcome: None,
    };
    let mut args = args.peekable();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screenshot" => {
                let path = args.next().ok_or("--screenshot needs a file name")?;
                launch.screenshot = Some(PathBuf::from(path));
            }
            "--dark" => launch.dark = true,
            "--sidebar" => launch.sidebar = Some(args.next().ok_or("--sidebar needs a view: history, chat, layers or hidden")?),
            "--settings" => {
                launch.settings = Some(
                    args.next()
                        .ok_or("--settings needs a page: general, canvas, agents, provider, models, models-search, cli, chat, images, image-service, permissions or mcp")?,
                );
            }
            "--actions" => launch.actions = true,
            "--generate-image" => launch.generate_image = true,
            "--background" => launch.background = true,
            "--activity" => launch.activity = true,
            "--points" => launch.points = true,
            "--attach" => launch.attach.push(args.next().ok_or("--attach needs a file")?.into()),
            "--find" => launch.find = Some(args.next().ok_or("--find needs the text to look for")?),
            "--private" => launch.private = true,
            "--no-update-check" => launch.no_update_check = true,
            "--update-prompt" => launch.update_prompt = true,
            "--update-required" => launch.update_required = true,
            "--update-collapsed" => launch.update_collapsed = true,
            // Human: how far the made-up download has come may follow, in per cent; without it, 42.
            "--update-download" => {
                let percent = args.next_if(|next| next.parse::<u8>().is_ok_and(|percent| percent <= 100)).and_then(|percent| percent.parse().ok());
                launch.update_download = Some(percent.unwrap_or(42));
            }
            "--home" => launch.home = true,
            // Human: the moment of the welcome film a picture shows may follow, in seconds; without it, its end.
            "--welcome" => {
                let at = args
                    .next_if(|next| next.parse::<f32>().is_ok_and(|at| (0.0..=theme::SPLASH_LENGTH).contains(&at)))
                    .and_then(|at| at.parse().ok());
                launch.welcome = Some(at.unwrap_or(theme::SPLASH_LENGTH));
            }
            "--ask" => launch.ask = Some(args.next().ok_or("--ask needs a message")?),
            "--assistant" => launch.assistant = Some(args.next().ok_or("--assistant needs an id, like code/claude-opus-5-5")?),
            "--export" => launch.export = Some(PathBuf::from(args.next().ok_or("--export needs a file name")?)),
            "--select" => launch.select = Some(args.next().ok_or("--select needs a layer's id")?),
            "--help" | "-h" => return Err("Quill".to_owned()),
            flag if flag.starts_with("--") => return Err(format!("unknown option {flag}")),
            file => launch.files.push(PathBuf::from(file)),
        }
    }
    Ok(launch)
}

#[expect(clippy::print_stderr, reason = "start-up errors go to the terminal that started the app")]
fn complain(message: &str) {
    eprintln!("{message}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arguments() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| (*s).to_owned()));
        let l = parse(&["a.pen", "--screenshot", "shot.png"]).unwrap();
        assert_eq!(l.files, vec![PathBuf::from("a.pen")]);
        assert_eq!(l.screenshot, Some(PathBuf::from("shot.png")));
        let l = parse(&["--screenshot", "shot.png", "--dark", "--sidebar", "history"]).unwrap();
        assert!(l.dark);
        assert_eq!(l.sidebar.as_deref(), Some("history"));
        assert!(parse(&["--sidebar"]).is_err());
        assert!(parse(&["--screenshot"]).is_err());
        assert!(parse(&["--bogus"]).is_err());
    }

    #[test]
    fn only_an_ordinary_run_looks_for_updates() {
        let parse = |a: &[&str]| parse_args(a.iter().map(|s| (*s).to_owned())).unwrap();
        assert!(parse(&[]).looks_for_updates());
        assert!(parse(&["a.pen", "--ask", "Draw a card"]).looks_for_updates());
        assert!(!parse(&["--no-update-check"]).looks_for_updates());
        assert!(!parse(&["--private"]).looks_for_updates());
        assert!(!parse(&["--background"]).looks_for_updates());
        assert!(!parse(&["--screenshot", "shot.png"]).looks_for_updates());
        assert!(!parse(&["--screenshot", "shot.png", "--export", "out.svg"]).looks_for_updates());
        assert!(parse(&["--screenshot", "shot.png", "--update-prompt"]).update_prompt);
        // A made-up download: how far it has come may follow; a file's name after it is still a file.
        assert_eq!(parse(&["--update-download"]).update_download, Some(42));
        assert_eq!(parse(&["--welcome"]).welcome, Some(theme::SPLASH_LENGTH));
        assert_eq!(parse(&["--screenshot", "shot.png", "--welcome", "3.5"]).welcome, Some(3.5));
        assert_eq!(parse(&[]).welcome, None);
        assert_eq!(parse(&["--screenshot", "shot.png", "--update-download", "80"]).update_download, Some(80));
        let named = parse(&["--update-download", "a.pen"]);
        assert_eq!((named.update_download, named.files.len()), (Some(42), 1));
        assert!(!parse(&["--update-download"]).looks_for_updates());
        let posed = parse(&["--screenshot", "shot.png", "--update-required", "--update-collapsed"]);
        assert!(posed.update_required && posed.update_collapsed);
    }
}
