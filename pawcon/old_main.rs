use chrono::Utc;
use iced::time;
use iced::widget::scrollable::{snap_to, Properties, Scrollbar, Scroller};
use iced::widget::{
    button, column, container, horizontal_space, progress_bar, radio, row, scrollable, slider,
    text, vertical_space,
};
use iced::Subscription;
use iced::{executor, theme, Alignment, Color};
use iced::{Application, Command, Element, Length, Settings, Theme};
use once_cell::sync::Lazy;
use serialport::SerialPort;
use serialport::{available_ports, SerialPortBuilder, SerialPortType};
use std::fs::File;
use std::ptr::null;
use std::time::{Duration, Instant, SystemTime};
use std::{io, thread};

static SCROLLABLE_ID: Lazy<scrollable::Id> = Lazy::new(scrollable::Id::unique);

pub fn main() -> iced::Result {
    ScrollableDemo::run(Settings::default())
}

struct ScrollableDemo {
    console_log: String,
    port: Option<SerialPortBuilder>,
    open_port: Option<Box<dyn SerialPort>>,
}

#[derive(Debug, Clone, Eq, PartialEq, Copy)]
enum Direction {
    Vertical,
    Horizontal,
    Multi,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchDirection(Direction),
    ScrollbarWidthChanged(u16),
    ScrollbarMarginChanged(u16),
    ScrollerWidthChanged(u16),
    ScrollToBeginning,
    QueryInfo, // update panel displaying state of pawpet
    RefreshCom,
    Scrolled(scrollable::RelativeOffset),
    Poll,
    Polled,
}

impl Application for ScrollableDemo {
    type Executor = executor::Default;
    type Message = Message;
    type Theme = Theme;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, Command<Message>) {
        (
            ScrollableDemo {
                console_log: String::new(),
                port: Option::None,
                open_port: Option::None,
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Pawcon")
    }

    fn subscription(&self) -> Subscription<Message> {
        time::every(Duration::from_millis(250 as u64)).map(|_| Message::Poll)
    }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::RefreshCom => {
                self.console_log.clear();
                println!("refreshing com");

                let ports = serialport::available_ports().expect("No ports found!");
                for p in ports {
                    match p.port_type {
                        SerialPortType::UsbPort(info) => {
                            self.console_log += format!(
                                "{}\nVID {:04x} PID {:04x}\n",
                                p.port_name, info.vid, info.pid
                            )
                            .as_ref();
                            // self.console_log += format!("serial: {}\nman: {}\nprod: {}\n\n",
                            //     info.serial_number.as_ref().map_or("n/a", String::as_str),
                            //     info.manufacturer.as_ref().map_or("n/a", String::as_str),
                            //     info.product.as_ref().map_or("n/a", String::as_str)).as_ref();

                            if info.serial_number.unwrap_or_default().starts_with("PAWPET") {
                                self.console_log +=
                                    format!("Connecting to {}\n", p.port_name).as_ref();
                                println!("found matching port");

                                // if self.open_port.is_some()
                                // {

                                // }

                                // force drop existing connection?
                                self.port = Option::None;

                                if self.open_port.is_some() {
                                    println!("dropping existing connection");
                                    drop(self.open_port.as_ref().unwrap());
                                }

                                self.open_port = Option::None;

                                println!("created connection");

                                self.port = Some(
                                    serialport::new(p.port_name, 9600)
                                        .timeout(Duration::from_millis(10)),
                                );

                                println!("opening connection");

                                // TODO this blocks, move to thread with single instance
                                // TODO spin progress bar or whatever
                                let open_port = self.port.as_ref().unwrap().clone().open();

                                if open_port.is_ok() {
                                    self.open_port = Some(open_port.unwrap());
                                    println!("connected");
                                } else {
                                    println!("connection failed {:?} ", open_port.err());
                                }
                                break;
                            }
                        }
                        _ => {
                            self.console_log += format!("{}\n", p.port_name).as_ref();
                        }
                    }

                    // TODO, use vid/pid to query actual usb info, validate that vid + pid match, as do manufacturer/product string

                    // log found com, store found com and usb info
                }
                scrollable::snap_to(SCROLLABLE_ID.clone(), scrollable::RelativeOffset::END)
            }
            Message::Poll => {
                if self.port.is_some() && self.open_port.is_some() {
                    let mut buffer: [u8; 64] = [0; 64];
                    let dt = Utc::now();

                    match self.open_port.as_mut().unwrap().read(&mut buffer) {
                        Ok(bytes) => {
                            let received = std::str::from_utf8(&buffer[..bytes]).unwrap();

                            let timestamped_strings = received.replace(
                                '\n',
                                format!("\n [{}] ", dt.format("%H:%M:%S").to_string()).as_ref(),
                            );

                            self.console_log += timestamped_strings.as_ref();

                            return scrollable::snap_to(
                                SCROLLABLE_ID.clone(),
                                scrollable::RelativeOffset::END,
                            );
                        }
                        Err(ref e) if e.kind() == io::ErrorKind::TimedOut => (),
                        Err(e) => {
                            eprintln!("{:?}", e);

                            self.console_log +=
                                format!("\n [{}] {:?}\n", dt.format("%H:%M:%S").to_string(), e).as_ref();
                            self.port = Option::None;
                        }
                    }
                }
                Command::none()
            }
            _ => Command::none(),
        }
    }

    fn view(&self) -> Element<Message> {
        let scroll_slider_controls = column![
            text("pawcon interface:"),
            button("refresh").padding(10).on_press(Message::RefreshCom)
        ]
        .spacing(10)
        .width(Length::Fill);

        let scroll_controls = row![scroll_slider_controls].spacing(20).width(Length::Fill);

        let scrollable_content: Element<Message> = Element::from(
            scrollable(
                column![text(&self.console_log),]
                    .width(Length::Fill)
                    .align_items(Alignment::Start)
                    .padding([40, 0, 40, 0])
                    .spacing(40),
            )
            .height(Length::Fill)
            .vertical_scroll(Properties::new())
            .id(SCROLLABLE_ID.clone())
            .on_scroll(Message::Scrolled),
        );

        let progress_bars: Element<Message> = progress_bar(0.0..=1.0, 0.5).into();

        let content: Element<Message> = column![scroll_controls, scrollable_content, progress_bars]
            .width(Length::Fill)
            .height(Length::Fill)
            .align_items(Alignment::Center)
            .spacing(10)
            .into();

        Element::from(
            container(content)
                .width(Length::Fill)
                .height(Length::Fill)
                .padding(40)
                .center_x()
                .center_y(),
        )
    }

    fn theme(&self) -> Self::Theme {
        Theme::Dark
    }
}

struct ScrollbarCustomStyle;

impl scrollable::StyleSheet for ScrollbarCustomStyle {
    type Style = Theme;

    fn active(&self, style: &Self::Style) -> Scrollbar {
        style.active(&theme::Scrollable::Default)
    }

    fn hovered(&self, style: &Self::Style) -> Scrollbar {
        style.hovered(&theme::Scrollable::Default)
    }

    fn hovered_horizontal(&self, style: &Self::Style) -> Scrollbar {
        Scrollbar {
            background: style.active(&theme::Scrollable::default()).background,
            border_radius: 0.0,
            border_width: 0.0,
            border_color: Default::default(),
            scroller: Scroller {
                color: Color::from_rgb8(250, 85, 134),
                border_radius: 0.0,
                border_width: 0.0,
                border_color: Default::default(),
            },
        }
    }
}

struct ProgressBarCustomStyle;

impl progress_bar::StyleSheet for ProgressBarCustomStyle {
    type Style = Theme;

    fn appearance(&self, style: &Self::Style) -> progress_bar::Appearance {
        progress_bar::Appearance {
            background: style.extended_palette().background.strong.color.into(),
            bar: Color::from_rgb8(250, 85, 134).into(),
            border_radius: 0.0,
        }
    }
}
