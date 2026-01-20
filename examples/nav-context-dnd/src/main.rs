// Copyright 2023 System76 <info@system76.com>
// SPDX-License-Identifier: MPL-2.0

//! Application API example

use std::collections::HashMap;

use cosmic::app::{Core, Settings, Task};
use cosmic::iced_core::Size;
use cosmic::widget::{menu, nav_bar};
use cosmic::{executor, iced, ApplicationExt, Element};

use cosmic::iced::clipboard::mime::{AllowedMimeTypes, AsMimeTypes};
use cosmic::widget::dnd_destination::dnd_destination_for_data;
use cosmic::widget::segmented_button::{self, ReorderEvent};
use std::borrow::Cow;
use std::convert::Infallible;

#[derive(Clone, Copy)]
pub enum Page {
    Page1,
    Page2,
    Page3,
    Page4,
}

impl Page {
    const fn as_str(self) -> &'static str {
        match self {
            Page::Page1 => "Page 1",
            Page::Page2 => "Page 2",
            Page::Page3 => "Page 3",
            Page::Page4 => "Page 4",
        }
    }
}

#[rustfmt::skip]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("RUST_LOG", "info,libcosmic::widget::tab_reorder=trace,wgpu_core=warn,wgpu_hal=warn,naga=warn");
    tracing_subscriber::fmt::init();
    let _ = tracing_log::LogTracer::init();

    let input = vec![
        (Page::Page1, "🖖 Hello from libcosmic.".into()),
        (Page::Page2, "🌟 This is an example application.".into()),
        (Page::Page3, "🚧 The libcosmic API is not stable yet.".into()),
        (Page::Page4, "🚀 Copy the source code and experiment today!".into()),
    ];

    let settings = Settings::default()
        .antialiasing(true)
        .client_decorations(true)
        .debug(false)
        .default_icon_theme("Pop")
        .default_text_size(16.0)
        .scale_factor(1.0)
        .size(Size::new(1024., 768.));

    cosmic::app::run::<App>(settings, input)?;
    Ok(())
}

const NAV_ITEM_MIME: &str = "application/x-cosmic-nav-item";

#[derive(Debug, Clone)]
pub struct NavItemMime;

impl AllowedMimeTypes for NavItemMime {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec![NAV_ITEM_MIME.to_string()])
    }
}

impl TryFrom<(Vec<u8>, String)> for NavItemMime {
    type Error = Infallible;

    fn try_from(_value: (Vec<u8>, String)) -> Result<Self, Self::Error> {
        Ok(NavItemMime)
    }
}

#[derive(Debug, Clone)]
pub struct DndText(pub String);

impl AllowedMimeTypes for DndText {
    fn allowed() -> Cow<'static, [String]> {
        Cow::Owned(vec!["text/plain".to_string()])
    }
}

impl AsMimeTypes for DndText {
    fn available(&self) -> Cow<'static, [String]> {
        Cow::Owned(vec!["text/plain".to_string()])
    }

    fn as_bytes(&self, mime_type: &str) -> Option<Cow<'static, [u8]>> {
        (mime_type == "text/plain").then(|| self.0.as_bytes().to_vec().into())
    }
}

impl TryFrom<(Vec<u8>, String)> for DndText {
    type Error = Infallible;

    fn try_from(value: (Vec<u8>, String)) -> Result<Self, Self::Error> {
        Ok(DndText(String::from_utf8(value.0).unwrap_or_default()))
    }
}

/// Messages that are used specifically by our [`App`].
#[derive(Clone, Debug)]
pub enum Message {
    NavMenuAction(NavMenuAction),
    SourceStarted,
    SourceFinished,
    SourceCancelled,
    ZoneHovered(f64, f64),
    ZoneDropped(String),
    NavReorder(ReorderEvent),
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum NavMenuAction {
    MoveUp(nav_bar::Id),
    MoveDown(nav_bar::Id),
    Delete(nav_bar::Id),
}

impl menu::Action for NavMenuAction {
    type Message = cosmic::Action<Message>;

    fn message(&self) -> Self::Message {
        cosmic::Action::App(Message::NavMenuAction(*self))
    }
}

/// The [`App`] stores application-specific state.
pub struct App {
    core: Core,
    nav_model: nav_bar::Model,
    dropped_text: String,
}

/// Implement [`cosmic::Application`] to integrate with COSMIC.
impl cosmic::Application for App {
    /// Default async executor to use with the app.
    type Executor = executor::Default;

    /// Argument received [`cosmic::Application::new`].
    type Flags = Vec<(Page, String)>;

    /// Message type specific to our [`App`].
    type Message = Message;

    /// The unique application ID to supply to the window manager.
    const APP_ID: &'static str = "org.cosmic.AppDemo";

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    /// Creates the application, and optionally emits task on initialize.
    fn init(core: Core, input: Self::Flags) -> (Self, Task<Self::Message>) {
        let mut nav_model = nav_bar::Model::default();

        for (title, content) in input {
            nav_model.insert().text(title.as_str()).data(content);
        }

        nav_model.activate_position(0);

        let mut app = App {
            core,
            nav_model,
            dropped_text: "Drop something here!".into(),
        };

        let command = app.update_title();

        (app, command)
    }

    /// Allows COSMIC to integrate with your application's [`nav_bar::Model`].
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav_model)
    }

    /// The context menu to display for the given nav bar item ID.
    fn nav_context_menu(
        &self,
        id: nav_bar::Id,
    ) -> Option<Vec<menu::Tree<cosmic::Action<Self::Message>>>> {
        Some(menu::items(
            &HashMap::new(),
            vec![
                menu::Item::Button("Move Up", None, NavMenuAction::MoveUp(id)),
                menu::Item::Button("Move Down", None, NavMenuAction::MoveDown(id)),
                menu::Item::Button("Delete", None, NavMenuAction::Delete(id)),
            ],
        ))
    }

    fn nav_bar(&self) -> Option<Element<'_, cosmic::Action<Self::Message>>> {
        let nav = cosmic::widget::segmented_button::vertical(&self.nav_model)
            .on_activate(|id| cosmic::Action::Cosmic(cosmic::app::Action::NavBar(id)))
            .on_context(|id| cosmic::Action::Cosmic(cosmic::app::Action::NavBarContext(id)))
            .enable_tab_drag(|id| {
                println!("Creating drag payload for {:?}", id);
                Some((NAV_ITEM_MIME.to_string(), Vec::new()))
            })
            .on_dnd_drop(|_id, _data: Option<NavItemMime>, _action| {
                // Dummy drop handler to force the widget to register NAV_ITEM_MIME as a valid destination.
                // This is a workaround for libcosmic not registering mimes for reorder-only widgets.
                cosmic::Action::None
            })
            .on_reorder(|event| cosmic::Action::App(Message::NavReorder(event)))
            .id(segmented_button::Id::new("nav_bar_id"));

        Some(
            cosmic::widget::container(nav)
                .width(iced::Length::Shrink)
                .height(iced::Length::Shrink)
                .max_width(280)
                .into(),
        )
    }

    /// Called when a navigation item is selected.
    fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<Self::Message> {
        self.nav_model.activate(id);
        self.update_title()
    }

    /// Handle application events here.
    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::NavMenuAction(message) => match message {
                NavMenuAction::Delete(id) => self.nav_model.remove(id),
                NavMenuAction::MoveUp(id) => {
                    if let Some(pos) = self.nav_model.position(id) {
                        if pos != 0 {
                            self.nav_model.position_set(id, pos - 1);
                        }
                    }
                }
                NavMenuAction::MoveDown(id) => {
                    if let Some(pos) = self.nav_model.position(id) {
                        self.nav_model.position_set(id, pos + 1);
                    }
                }
            },
            Message::NavReorder(event) => {
                println!("NavReorder TRIGGERED: {:?}", event);
                println!("  Before: {:?}", self.nav_model.iter().collect::<Vec<_>>());
                if self
                    .nav_model
                    .reorder(event.dragged, event.target, event.position)
                {
                    println!("  Reorder SUCCESS");
                    println!("  After: {:?}", self.nav_model.iter().collect::<Vec<_>>());
                } else {
                    println!("  Reorder FAILED");
                }
            }
            Message::SourceStarted => {
                println!("Source started");
            }
            Message::SourceFinished => {
                println!("Source finished");
            }
            Message::SourceCancelled => {
                println!("Source cancelled");
            }
            Message::ZoneHovered(x, y) => {
                println!("Zone hovered at {x}, {y}");
            }
            Message::ZoneDropped(data) => {
                println!("Dropped: {data}");
                self.dropped_text = format!("Dropped: {data}");
            }
        }

        Task::none()
    }

    /// Creates a view after each update.
    fn view(&self) -> Element<'_, Self::Message> {
        let page_content = self
            .nav_model
            .active_data::<String>()
            .map_or("No page selected", String::as_str);

        let text = cosmic::widget::text(page_content);

        let centered = cosmic::widget::container(text)
            .width(iced::Length::Fill)
            .height(iced::Length::Shrink)
            .align_x(iced::Alignment::Center)
            .align_y(iced::Alignment::Center);

        let source = cosmic::widget::dnd_source(
            cosmic::widget::container(cosmic::widget::text("Drag me!"))
                .padding(20)
                .class(cosmic::theme::Container::Card),
        )
        .drag_content(|| DndText("Hello from source!".to_string()))
        .on_start(Some(Message::SourceStarted))
        .on_finish(Some(Message::SourceFinished))
        .on_cancel(Some(Message::SourceCancelled));

        let destination = dnd_destination_for_data::<DndText, Message>(
            cosmic::widget::container(cosmic::widget::text(&self.dropped_text))
                .padding(50)
                .class(cosmic::theme::Container::Card)
                .width(iced::Length::Fill)
                .align_x(iced::Alignment::Center),
            |data, _action| {
                if let Some(data) = data {
                    Message::ZoneDropped(data.0)
                } else {
                    // Handle wrong type or error if needed, for simplicity just ignore
                    Message::SourceCancelled
                }
            },
        )
        .on_drop(|_x, _y| Message::SourceFinished) // This is triggered when drop happens but we primarily use the data callback above
        .on_motion(|x, y| Message::ZoneHovered(x, y));

        let content = cosmic::widget::column()
            .push(centered)
            .push(source)
            .push(destination)
            .spacing(20)
            .align_x(iced::Alignment::Center)
            .padding(20);

        Element::from(content)
    }
}

impl App
where
    Self: cosmic::Application,
{
    fn active_page_title(&mut self) -> &str {
        self.nav_model
            .text(self.nav_model.active())
            .unwrap_or("Unknown Page")
    }

    fn update_title(&mut self) -> Task<Message> {
        let header_title = self.active_page_title().to_owned();
        let window_title = format!("{header_title} — COSMIC AppDemo");
        self.set_header_title(header_title);
        if let Some(win_id) = self.core.main_window_id() {
            self.set_window_title(window_title, win_id)
        } else {
            Task::none()
        }
    }
}
