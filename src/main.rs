use std::{
    env, fs, io,
    path::{Path, PathBuf},
};

use ffmpeg_next as ffmpeg;

use iced::{
    Color, Element, Event, Length, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    color, event,
    keyboard::{self, Key, key},
    task::{self},
    widget, window,
};

mod utils;
use crate::utils::*;

mod paths;
use paths::*;

mod media;
use media::*;

#[derive(Debug, Clone)]
enum Message {
    InputChange(String),
    OutputChange(String, bool),

    PickInput,
    PickOutput,
    InputPicked(Option<PathBuf>),
    OutputPicked(Option<PathBuf>),

    StartChange(f64),
    EndChange(f64),
    EagerStartChange(f64),
    EagerEndChange(f64),

    ToggleVideo,
    ToggleAudio,
    ToggleSubs,
    ToggleExtraStreams,

    Submitted,

    Update,

    LoadedStartPreview(Result<(widget::image::Handle, u64), PreviewError>),
    LoadedEndPreview(Result<(widget::image::Handle, u64), PreviewError>),

    AllocatedStartPreview(Result<widget::image::Allocation, widget::image::Error>),
    AllocatedEndPreview(Result<widget::image::Allocation, widget::image::Error>),

    PlayStartPreview,
    PlayEndPreview,

    Event(Event),

    Instantiate,
    InstantiateFinished(Result<(), String>),
}

#[derive(Debug, Default)]
struct PreviewState {
    last_start: Preview,
    last_end: Preview,

    last_start_hash: u64,
    last_end_hash: u64,

    start: Option<widget::image::Allocation>,
    end: Option<widget::image::Allocation>,

    start_task_handle: Option<task::Handle>,
    end_task_handle: Option<task::Handle>,

    player: media::Player,
}

#[derive(Debug, Default)]
struct State {
    media: Media,

    input_changed: bool,
    input_exists: bool,

    input_length: f64,

    number_changed: bool,

    previews: PreviewState,

    output_is_generated: bool,
    output_folder_exists: bool,

    error: String,
    status: String,
}

impl State {
    fn new() -> (Self, Task<Message>) {
        ffmpeg::init().unwrap();

        let state = State::default();

        // Uses the first argument as the input file path,
        // and creates the output file path from it
        let mut args = env::args();
        if let Some(str) = args.nth(1) {
            (
                state,
                Task::done(Message::InputChange(str)).chain(Task::done(Message::Update)),
            )
        } else {
            (state, Task::none())
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::InputChange(str) => {
                self.media.input = str;
                self.input_changed = true;
                match fs::metadata(&self.media.input) {
                    Ok(metadata) => self.input_exists = metadata.is_file(),
                    Err(e) if e.kind() == io::ErrorKind::NotFound => self.input_exists = false,
                    Err(e) => eprintln!(
                        "failed to check if input '{}' exists: {e}",
                        self.media.input
                    ),
                }
            }
            Message::OutputChange(str, is_generated) => {
                self.media.output = str;
                self.output_is_generated = is_generated;
                if let Some(path) = Path::new(&self.media.output).parent()
                    && let Ok(exists) = path
                        .try_exists()
                        .inspect_err(|e| eprintln!("failed to check if output path exists: {e}"))
                {
                    self.output_folder_exists = exists;
                }
            }
            Message::StartChange(val) => {
                self.media.start = val;
                self.number_changed = true;
            }
            Message::EndChange(val) => {
                self.media.end = val;
                self.number_changed = true;
            }

            Message::EagerStartChange(val) => {
                self.media.start = val;
                self.number_changed = true;
                return self.check_inputs();
            }
            Message::EagerEndChange(val) => {
                self.media.end = val;
                self.number_changed = true;
                return self.check_inputs();
            }

            Message::PickInput => return Task::perform(pick_file(), Message::InputPicked),
            Message::PickOutput => return Task::perform(pick_folder(), Message::OutputPicked),
            Message::InputPicked(opt) => {
                if let Some(path) = opt
                    && let Some(str) = path.to_str()
                {
                    return Task::done(Message::InputChange(str.to_owned()))
                        .chain(Task::done(Message::Update));
                }
            }
            Message::OutputPicked(opt) => {
                if let Some(mut path) = opt {
                    // push instead of setting filename
                    // since picked folder is interpreted as the filename here
                    path.push(
                        Path::new(&self.media.output)
                            .file_name()
                            .unwrap_or_default(),
                    );
                    if let Some(str) = path.to_str() {
                        return Task::done(Message::OutputChange(str.to_owned(), false));
                    }
                }
            }

            Message::Submitted => return self.check_inputs(),
            Message::Update => return self.check_inputs(),

            Message::ToggleVideo => self.media.use_video.toggle(),
            Message::ToggleAudio => self.media.use_audio.toggle(),
            Message::ToggleSubs => self.media.use_subs.toggle(),
            Message::ToggleExtraStreams => self.media.use_extra_streams.toggle(),

            Message::LoadedStartPreview(Ok((handle, hash))) => {
                self.previews.last_start_hash = hash;

                return widget::image::allocate(handle).map(Message::AllocatedStartPreview);
            }
            Message::LoadedEndPreview(Ok((handle, hash))) => {
                self.previews.last_end_hash = hash;

                return widget::image::allocate(handle).map(Message::AllocatedEndPreview);
            }
            Message::LoadedStartPreview(Err(e)) | Message::LoadedEndPreview(Err(e)) => {
                if e != PreviewError::SameHash {
                    eprintln!("failed to load preview: {e}")
                }
            }

            Message::AllocatedStartPreview(Ok(allocation)) => {
                self.previews.start = Some(allocation)
            }
            Message::AllocatedEndPreview(Ok(allocation)) => self.previews.end = Some(allocation),
            Message::AllocatedStartPreview(Err(e)) | Message::AllocatedEndPreview(Err(e)) => {
                eprintln!("failed to allocate preview: {e}")
            }

            Message::PlayStartPreview => {
                self.previews
                    .player
                    .toggle_preview(&self.media, self.media.start);
            }
            Message::PlayEndPreview => {
                self.previews
                    .player
                    .toggle_preview(&self.media, self.media.end - 5.0);
            }

            Message::Event(Event::Keyboard(keyboard::Event::KeyPressed {
                key, modifiers, ..
            })) => match key.as_ref() {
                // input field cycling
                Key::Named(key::Named::Tab) => {
                    if modifiers.shift() {
                        return widget::operation::focus_previous();
                    } else {
                        return widget::operation::focus_next();
                    }
                }

                Key::Named(key::Named::ArrowRight) | Key::Character("l") => {
                    return if modifiers.shift() {
                        Task::done(Message::EagerEndChange(self.media.end + 5.0))
                    } else {
                        Task::done(Message::EagerStartChange(self.media.start + 5.0))
                    };
                }
                Key::Named(key::Named::ArrowLeft) | Key::Character("h") => {
                    return if modifiers.shift() {
                        Task::done(Message::EagerEndChange(self.media.end - 5.0))
                    } else {
                        Task::done(Message::EagerStartChange(self.media.start - 5.0))
                    };
                }

                Key::Named(key::Named::ArrowUp) | Key::Character("k") => {
                    return if modifiers.shift() {
                        Task::done(Message::EagerEndChange(self.media.end + 10.0))
                    } else {
                        Task::done(Message::EagerStartChange(self.media.start + 10.0))
                    };
                }
                Key::Named(key::Named::ArrowDown) | Key::Character("j") => {
                    return if modifiers.shift() {
                        Task::done(Message::EagerEndChange(self.media.end - 10.0))
                    } else {
                        Task::done(Message::EagerStartChange(self.media.start - 10.0))
                    };
                }

                Key::Character("v") => return Task::done(Message::ToggleVideo),
                Key::Character("a") => return Task::done(Message::ToggleAudio),
                Key::Character("s") => return Task::done(Message::ToggleSubs),
                Key::Character("e") => return Task::done(Message::ToggleExtraStreams),

                Key::Character("i") | Key::Character("f") => {
                    return Task::done(Message::PickInput);
                }
                Key::Character("o") | Key::Character("d") => {
                    return Task::done(Message::PickOutput);
                }

                Key::Character("p") => {
                    return Task::done(if modifiers.shift() {
                        Message::PlayEndPreview
                    } else {
                        Message::PlayStartPreview
                    });
                }

                Key::Character("q") => {
                    return window::latest().and_then(window::close);
                }

                Key::Named(key::Named::Enter) => {
                    if modifiers.shift() {
                        return Task::done(Message::Instantiate);
                    }
                }

                // ignore unbound keys
                _ => {}
            },
            // ignore all non-keyboard events
            Message::Event(_) => {}

            Message::Instantiate => {
                self.error.clear();
                self.status = "Loading...".to_string();
                return self.instantiate();
            }
            Message::InstantiateFinished(result) => match result {
                Ok(()) => {
                    self.status = "Finished".to_string();
                    return window::latest().and_then(window::close);
                }
                Err(e) => self.error = e,
            },
        }

        Task::none()
    }

    fn view(&self) -> Element<'_, Message> {
        let input_field = widget::text_input("input file", &self.media.input)
            .on_input(Message::InputChange)
            .on_submit(Message::Submitted);
        let input_picker = widget::button("pick file")
            .on_press(Message::PickInput)
            .style(if self.input_exists {
                widget::button::primary
            } else {
                widget::button::warning
            });

        let start_slider = widget::slider(
            0_f64..=self.media.end - 1.0,
            self.media.start,
            Message::EagerStartChange,
        )
        .default(0);
        let start_field = widget::text_input("start", &self.media.start.to_string())
            .on_input(|str| Message::StartChange(str.parse().unwrap_or_default()))
            .width(200)
            .on_submit(Message::Submitted);

        let end_slider = widget::slider(
            self.media.start + 1.0..=self.input_length,
            self.media.end,
            Message::EagerEndChange,
        )
        .default(self.input_length);
        let end_field = widget::text_input("end", &self.media.end.to_string())
            .on_input(|str| Message::EndChange(str.parse().unwrap_or_default()))
            .width(200)
            .on_submit(Message::Submitted);

        let output_field = widget::text_input("output file", &self.media.output)
            .on_input(|str| Message::OutputChange(str, false))
            .on_submit(Message::Submitted);
        let output_picker = widget::button("pick folder")
            .on_press(Message::PickOutput)
            .style(if self.output_folder_exists {
                widget::button::primary
            } else {
                widget::button::warning
            });

        let video_checkbox = widget::checkbox(self.media.use_video)
            .on_toggle(|_| Message::ToggleVideo)
            .label("video");
        let audio_checkbox = widget::checkbox(self.media.use_audio)
            .on_toggle(|_| Message::ToggleAudio)
            .label("audio");
        let subs_checkbox = widget::checkbox(self.media.use_subs)
            .on_toggle(|_| Message::ToggleSubs)
            .label("subtitles");
        let extra_streams_checkbox = widget::checkbox(self.media.use_extra_streams)
            .on_toggle(|_| Message::ToggleExtraStreams)
            .label("extra streams");

        let preview_row = if self.media.use_video
            && let Some(start) = &self.previews.start
            && let Some(end) = &self.previews.end
        {
            widget::row![
                widget::image(start.handle())
                    .width(Length::Fill)
                    .height(Length::Fill),
                widget::image(end.handle())
                    .width(Length::Fill)
                    .height(Length::Fill)
            ]
        } else {
            widget::row![]
        };

        let status_display = if !self.error.is_empty() {
            widget::row![widget::text(&self.error).style(widget::text::danger)]
        } else if !self.status.is_empty() {
            widget::row![widget::text(&self.status).style(widget::text::primary)]
        } else {
            widget::row![]
        };

        let instantiate_button = widget::button("Instantiate!").on_press(Message::Instantiate);
        let duration_string = format!("Duration: {} seconds", self.media.end - self.media.start);

        let start_play_button =
            widget::button("play start preview").on_press(Message::PlayStartPreview);
        let end_play_button = widget::button("play end preview").on_press(Message::PlayEndPreview);

        #[rustfmt::skip]
        return widget::column![
            widget::row![input_field, input_picker],

            widget::row![widget::text("Start time (seconds):  "), start_field, start_slider]
                .align_y(Vertical::Center),

            widget::row![widget::text("End time (seconds):    "), end_field, end_slider]
                .align_y(Vertical::Center),

            widget::row![video_checkbox, audio_checkbox, subs_checkbox, extra_streams_checkbox]
                .spacing(100)
                .align_y(Vertical::Center),

            widget::row![output_field, output_picker],

            preview_row,

            status_display,

            widget::row![
                start_play_button,
                widget::text("Press Shift-Enter, or:"), instantiate_button, widget::text(duration_string),
                end_play_button,
                ]
                .spacing(10)
                .align_y(Vertical::Center)
        ]
        .spacing(20)
        .align_x(Horizontal::Center)
        .into();
    }

    fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::Event)
    }

    fn check_inputs(&mut self) -> Task<Message> {
        let mut tasks = Vec::new();

        if self.number_changed {
            self.clamp_numbers();
            if !self.input_changed {
                tasks.push(self.create_preview_images());
            }

            self.number_changed = false;
        }
        if self.input_changed {
            match self.update_from_input() {
                Err(e) => eprintln!("failed to inspect input media '{}': {e}", self.media.input),
                Ok(task) => {
                    tasks.push(task.chain(Task::done(Message::Update)));
                }
            }

            self.input_changed = false;
        } else if self.media.output.is_empty()
            && !self.output_is_generated
            && !self.media.input.is_empty()
        {
            tasks.push(self.generate_output_path());
        }

        Task::batch(tasks)
    }

    fn clamp_numbers(&mut self) {
        if self.media.start < 0.0 {
            self.media.start = 0.0;
        }
        if self.media.end < 0.0 {
            self.media.end = 0.0;
        }

        if self.media.end > self.input_length {
            self.media.end = self.input_length;
        }

        if self.media.start > self.media.end {
            self.media.start = self.media.end;
        }

        if self.media.end < self.media.start {
            self.media.end = self.media.start;
        }
    }

    fn update_from_input(&mut self) -> Result<Task<Message>, ffmpeg::Error> {
        if !self.input_exists {
            eprintln!("input_exists is set to false, not attempting to update from input");
            return Err(ffmpeg::Error::Unknown);
        }

        self.input_length = self.media.update_video_params()?;

        Ok(Task::batch([
            // Set the end to the duration of the video
            Task::done(Message::EndChange(self.input_length)),
            if self.media.output.is_empty() || self.output_is_generated {
                // Generate a output path if there is none from user input
                self.generate_output_path()
            } else {
                Task::none()
            },
        ]))
    }

    fn generate_output_path(&mut self) -> Task<Message> {
        let input_path = PathBuf::from(&self.media.input);

        Task::perform(modify_path(input_path), |path| {
            Message::OutputChange(
                path.into_os_string().into_string().unwrap_or_default(),
                true,
            )
        })
    }

    fn instantiate(&self) -> Task<Message> {
        Task::perform(self.media.clone().create(), Message::InstantiateFinished)
    }

    /// makes a batch of tasks to create start and end preview images
    /// no effect if use_video is false
    fn create_preview_images(&mut self) -> Task<Message> {
        if !self.media.use_video {
            return Task::none();
        }

        let start = Preview {
            seek: self.media.start,
            input: self.media.input.clone(),
        };
        let end = Preview {
            seek: // seek slightly before the end of the video to get a frame
                if self.media.end > self.input_length - 0.1 {
                    self.media.end - 0.5
                } else {
                    self.media.end
                },
            input: self.media.input.clone(),
        };

        Task::batch([
            if start == self.previews.last_start {
                // No need to reload the same image
                Task::none()
            } else {
                self.previews.last_start = start.clone();
                let (task, handle) = Task::perform(
                    start.decode_image(self.previews.last_start_hash),
                    Message::LoadedStartPreview,
                )
                .abortable();

                if let Some(handle) = &self.previews.start_task_handle {
                    handle.abort();
                }

                self.previews.start_task_handle = Some(handle);

                task
            },
            if end == self.previews.last_end {
                // No need to reload the same image
                Task::none()
            } else {
                self.previews.last_end = end.clone();
                let (task, handle) = Task::perform(
                    end.decode_image(self.previews.last_end_hash),
                    Message::LoadedEndPreview,
                )
                .abortable();

                if let Some(handle) = &self.previews.end_task_handle {
                    handle.abort();
                }

                self.previews.end_task_handle = Some(handle);

                task
            },
        ])
    }
}

fn main() -> Result<(), iced::Error> {
    iced::application(State::new, State::update, State::view)
        .subscription(State::subscription)
        .theme(Theme::custom(
            "custom",
            iced::theme::Palette {
                background: color!(0x080808),
                text: Color::WHITE,
                primary: color!(0x00ffff),
                success: color!(0x00ff00),
                warning: color!(0x880000),
                danger: color!(0xff0000),
            },
        ))
        .window_size((1000, 600))
        .run()?;

    Ok(())
}
