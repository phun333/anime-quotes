use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    DefaultTerminal, Frame,
    layout::Rect,
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph},
};
use ratatui_image::{
    FilterType, Resize, StatefulImage, picker::Picker, protocol::StatefulProtocol,
};
use serde::Deserialize;
use std::fs;
use std::io;

#[derive(Debug, Deserialize)]
struct AnimeQuote {
    japanese: String,
    #[serde(default)]
    romaji: Option<String>,
    anime: String,
    character: String,
    quote: String,
    image: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AnimeData {
    quotes: Vec<AnimeQuote>,
}

const DEFAULT_ASCII_TARGET_WIDTH: u32 = 30;
const DEFAULT_ASCII_CHAR_ASPECT: f32 = 0.5;
const DEFAULT_ASCII_DETAIL_X: u32 = 2;
const DEFAULT_ASCII_DETAIL_Y: u32 = 2;
const DEFAULT_ASCII_GRADIENT: &str =
    r#"$@B%8&WM#*oahkbdpqwmZO0QLCJUYXzcvunxrjft/\|()1{}[]?-_+~<>i!lI;:,"^`'. "#;
const DEFAULT_SHOW_INSTRUCTIONS: bool = true;

#[derive(Debug, Deserialize)]
struct ConfigRoot {
    #[serde(default)]
    ui: UiConfig,
}

impl Default for ConfigRoot {
    fn default() -> Self {
        Self {
            ui: UiConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct UiConfig {
    #[serde(default = "default_show_instructions")]
    show_instructions: bool,
    #[serde(default)]
    ascii: AsciiConfig,
    #[serde(default)]
    colors: ColorConfig,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            show_instructions: DEFAULT_SHOW_INSTRUCTIONS,
            ascii: AsciiConfig::default(),
            colors: ColorConfig::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct AsciiConfig {
    #[serde(default = "default_ascii_target_width")]
    target_width: u32,
    #[serde(default = "default_ascii_char_aspect")]
    char_aspect: f32,
    #[serde(default = "default_ascii_gradient")]
    gradient: String,
    #[serde(default = "default_ascii_detail_x")]
    detail_x: u32,
    #[serde(default = "default_ascii_detail_y")]
    detail_y: u32,
}

impl Default for AsciiConfig {
    fn default() -> Self {
        Self {
            target_width: DEFAULT_ASCII_TARGET_WIDTH,
            char_aspect: DEFAULT_ASCII_CHAR_ASPECT,
            gradient: DEFAULT_ASCII_GRADIENT.to_string(),
            detail_x: DEFAULT_ASCII_DETAIL_X,
            detail_y: DEFAULT_ASCII_DETAIL_Y,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ColorConfig {
    #[serde(default = "default_color_anime")]
    anime: String,
    #[serde(default = "default_color_character")]
    character: String,
    #[serde(default = "default_color_japanese")]
    japanese: String,
    #[serde(default = "default_color_romaji")]
    romaji: String,
    #[serde(default = "default_color_quote")]
    quote: String,
    #[serde(default = "default_color_count")]
    count: String,
    #[serde(default = "default_color_instructions")]
    instructions: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            anime: default_color_anime(),
            character: default_color_character(),
            japanese: default_color_japanese(),
            romaji: default_color_romaji(),
            quote: default_color_quote(),
            count: default_color_count(),
            instructions: default_color_instructions(),
        }
    }
}

fn default_ascii_target_width() -> u32 {
    DEFAULT_ASCII_TARGET_WIDTH
}

fn default_ascii_char_aspect() -> f32 {
    DEFAULT_ASCII_CHAR_ASPECT
}

fn default_ascii_gradient() -> String {
    DEFAULT_ASCII_GRADIENT.to_string()
}

fn default_ascii_detail_x() -> u32 {
    DEFAULT_ASCII_DETAIL_X
}

fn default_ascii_detail_y() -> u32 {
    DEFAULT_ASCII_DETAIL_Y
}

fn default_show_instructions() -> bool {
    DEFAULT_SHOW_INSTRUCTIONS
}

fn default_color_anime() -> String {
    "yellow".to_string()
}

fn default_color_character() -> String {
    "cyan".to_string()
}

fn default_color_japanese() -> String {
    "green".to_string()
}

fn default_color_romaji() -> String {
    "magenta".to_string()
}

fn default_color_quote() -> String {
    "white".to_string()
}

fn default_color_count() -> String {
    "gray".to_string()
}

fn default_color_instructions() -> String {
    "blue".to_string()
}

impl UiConfig {
    fn load_from_file(path: &str) -> Self {
        match fs::read_to_string(path) {
            Ok(content) => toml::from_str::<ConfigRoot>(&content)
                .map(|root| root.ui)
                .unwrap_or_else(|error| {
                    eprintln!("failed to parse {path}: {error}");
                    UiConfig::default()
                }),
            Err(error) => {
                eprintln!("failed to read {path}: {error}");
                UiConfig::default()
            }
        }
    }
}

#[derive(Clone, Debug)]
struct AsciiSettings {
    base_width: u32,
    char_aspect: f32,
    #[allow(dead_code)]
    gradient: Vec<char>,
    detail_x: u32,
    detail_y: u32,
}

impl AsciiSettings {
    fn target_dimensions(&self) -> (u16, u16) {
        let width = self.base_width.clamp(1, u16::MAX as u32) as u16;
        let height = ((self.base_width as f32) * self.char_aspect.max(0.1))
            .round()
            .clamp(1.0, u16::MAX as f32) as u16;
        (width.max(1), height.max(1))
    }

    fn resize_strategy(&self) -> Resize {
        if self.detail_x > 1 || self.detail_y > 1 {
            Resize::Scale(Some(FilterType::CatmullRom))
        } else {
            Resize::Fit(Some(FilterType::CatmullRom))
        }
    }
}

impl AsciiConfig {
    fn to_settings(&self) -> AsciiSettings {
        let mut gradient: Vec<char> = if self.gradient.trim().is_empty() {
            DEFAULT_ASCII_GRADIENT.chars().collect()
        } else {
            self.gradient.chars().collect()
        };

        if gradient.is_empty() {
            gradient = DEFAULT_ASCII_GRADIENT.chars().collect();
        }

        let char_aspect = if self.char_aspect <= 0.0 {
            DEFAULT_ASCII_CHAR_ASPECT
        } else {
            self.char_aspect
        };

        AsciiSettings {
            base_width: self.target_width.max(1),
            char_aspect,
            gradient,
            detail_x: self.detail_x.max(1),
            detail_y: self.detail_y.max(1),
        }
    }
}

#[derive(Clone, Debug)]
struct Palette {
    anime: Color,
    character: Color,
    japanese: Color,
    romaji: Color,
    quote: Color,
    count: Color,
    instructions: Color,
}

impl Default for Palette {
    fn default() -> Self {
        Self {
            anime: Color::Yellow,
            character: Color::Cyan,
            japanese: Color::Green,
            romaji: Color::Magenta,
            quote: Color::White,
            count: Color::Gray,
            instructions: Color::Blue,
        }
    }
}

impl ColorConfig {
    fn to_palette(&self) -> Palette {
        Palette {
            anime: parse_color_or_default(&self.anime, Color::Yellow),
            character: parse_color_or_default(&self.character, Color::Cyan),
            japanese: parse_color_or_default(&self.japanese, Color::Green),
            romaji: parse_color_or_default(&self.romaji, Color::Magenta),
            quote: parse_color_or_default(&self.quote, Color::White),
            count: parse_color_or_default(&self.count, Color::Gray),
            instructions: parse_color_or_default(&self.instructions, Color::Blue),
        }
    }
}

fn parse_color_or_default(value: &str, default: Color) -> Color {
    parse_color(value).unwrap_or(default)
}

fn parse_color(value: &str) -> Option<Color> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }

    if let Some(rgb) = parse_hex_color(trimmed) {
        return Some(rgb);
    }

    match trimmed.to_lowercase().as_str() {
        "black" => Some(Color::Black),
        "red" => Some(Color::Red),
        "green" => Some(Color::Green),
        "yellow" => Some(Color::Yellow),
        "blue" => Some(Color::Blue),
        "magenta" => Some(Color::Magenta),
        "cyan" => Some(Color::Cyan),
        "white" => Some(Color::White),
        "gray" | "grey" => Some(Color::Gray),
        "darkgray" | "darkgrey" => Some(Color::DarkGray),
        "lightgray" | "lightgrey" => Some(Color::Gray),
        _ => None,
    }
}

fn parse_hex_color(value: &str) -> Option<Color> {
    let hex = value.strip_prefix('#')?;
    match hex.len() {
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()?;
            Some(Color::Rgb(r * 17, g * 17, b * 17))
        }
        _ => None,
    }
}

struct ImageSlot {
    protocol: StatefulProtocol,
}

const IMAGE_TOP_PADDING: u16 = 2;
const IMAGE_TEXT_GAP: u16 = 1;

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

pub struct App {
    quotes: Vec<AnimeQuote>,
    image_cache: Vec<Option<ImageSlot>>,
    image_resize: Resize,
    image_width: u16,
    image_height: u16,
    palette: Palette,
    show_instructions: bool,
    current_index: usize,
    exit: bool,
}

impl Default for App {
    fn default() -> Self {
        let quotes = Self::load_quotes().unwrap_or_default();
        let ui_config = UiConfig::load_from_file("config.toml");
        let ascii_settings = ui_config.ascii.to_settings();
        let palette = ui_config.colors.to_palette();
        let (image_width, image_height) = ascii_settings.target_dimensions();
        let image_resize = ascii_settings.resize_strategy();

        let picker = match Picker::from_query_stdio() {
            Ok(picker) => picker,
            Err(error) => {
                eprintln!("failed to detect terminal graphics capabilities: {error}");
                Picker::from_fontsize((10, 20))
            }
        };

        let image_cache = quotes
            .iter()
            .map(|quote| {
                quote
                    .image
                    .as_deref()
                    .and_then(|path| match image::open(path) {
                        Ok(image) => {
                            let protocol = picker.new_resize_protocol(image);
                            Some(ImageSlot { protocol })
                        }
                        Err(error) => {
                            eprintln!("failed to load image from {path}: {error}");
                            None
                        }
                    })
            })
            .collect();
        Self {
            quotes,
            image_cache,
            image_resize,
            image_width,
            image_height,
            palette,
            show_instructions: ui_config.show_instructions,
            current_index: 0,
            exit: false,
        }
    }
}

impl App {
    fn load_quotes() -> Result<Vec<AnimeQuote>, Box<dyn std::error::Error>> {
        let content = fs::read_to_string("anime.toml")?;
        let data: AnimeData = toml::from_str(&content)?;
        Ok(data.quotes)
    }

    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        let title = Line::from(" Anime Quotes ".bold());
        let mut block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);
        if self.show_instructions {
            block = block.title_bottom(self.instructions_line().centered());
        }

        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.width == 0 || inner.height == 0 {
            return;
        }

        let reserved_vertical = IMAGE_TOP_PADDING + IMAGE_TEXT_GAP;
        let available_for_image = inner.height.saturating_sub(reserved_vertical);
        let image_height = self.image_height.min(available_for_image);
        let text_height = inner
            .height
            .saturating_sub(IMAGE_TOP_PADDING + image_height + IMAGE_TEXT_GAP);

        if image_height > 0 {
            let image_width = self.image_width.min(inner.width);
            let image_x = inner.x + (inner.width.saturating_sub(image_width)) / 2;
            let image_area = Rect {
                x: image_x,
                y: inner.y + IMAGE_TOP_PADDING,
                width: image_width,
                height: image_height,
            };

            let resize = self.image_resize.clone();
            if let Some(slot) = self.current_image_mut() {
                let widget = StatefulImage::<StatefulProtocol>::new().resize(resize);
                frame.render_stateful_widget(widget, image_area, &mut slot.protocol);
                if let Some(result) = slot.protocol.last_encoding_result() {
                    if let Err(error) = result {
                        eprintln!("failed to encode image: {error}");
                    }
                }
            } else {
                let placeholder = Paragraph::new(Text::from(Line::from(Span::styled(
                    "Image not available",
                    Style::default().fg(Color::Gray),
                ))))
                .alignment(ratatui::layout::Alignment::Center);
                frame.render_widget(placeholder, image_area);
            }
        }

        if text_height == 0 {
            return;
        }

        let text_area = Rect {
            x: inner.x,
            y: inner.y + IMAGE_TOP_PADDING + image_height + IMAGE_TEXT_GAP,
            width: inner.width,
            height: text_height,
        };

        let mut lines: Vec<Line> = Vec::new();

        if let Some(quote) = self.current_quote() {
            let anime_style = Style::default().fg(self.palette.anime).bold();
            let character_style = Style::default().fg(self.palette.character).bold();
            let japanese_style = Style::default().fg(self.palette.japanese).bold();
            let romaji_style = Style::default().fg(self.palette.romaji);
            let quote_style = Style::default().fg(self.palette.quote).italic();
            let count_style = Style::default().fg(self.palette.count);

            lines.extend(vec![
                Line::from(vec![
                    Span::raw("Anime: "),
                    Span::styled(quote.anime.clone(), anime_style),
                ]),
                Line::from(vec![
                    Span::raw("Character: "),
                    Span::styled(quote.character.clone(), character_style),
                ]),
                Line::from(""),
                Line::from(vec![
                    Span::raw("Japanese: "),
                    Span::styled(quote.japanese.clone(), japanese_style),
                ]),
            ]);

            if let Some(romaji) = &quote.romaji {
                lines.push(Line::from(vec![
                    Span::raw("Romaji: "),
                    Span::styled(romaji.clone(), romaji_style),
                ]));
            }

            lines.extend(vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("\""),
                    Span::styled(quote.quote.clone(), quote_style),
                    Span::raw("\""),
                ]),
                Line::from(""),
                Line::from(vec![Span::styled(
                    format!("({}/{})", self.current_index + 1, self.quotes.len()),
                    count_style,
                )]),
            ]);
        } else {
            lines.push(Line::from(Span::styled(
                "No quotes found!",
                Style::default().fg(Color::Red),
            )));
            lines.push(Line::from(Span::styled(
                "Make sure anime.toml exists in project root.",
                Style::default().fg(Color::Gray),
            )));
        }

        let paragraph =
            Paragraph::new(Text::from(lines)).alignment(ratatui::layout::Alignment::Center);
        frame.render_widget(paragraph, text_area);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Left => self.previous_quote(),
            KeyCode::Right => self.next_quote(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn next_quote(&mut self) {
        if !self.quotes.is_empty() {
            self.current_index = (self.current_index + 1) % self.quotes.len();
        }
    }

    fn previous_quote(&mut self) {
        if !self.quotes.is_empty() {
            self.current_index = if self.current_index == 0 {
                self.quotes.len() - 1
            } else {
                self.current_index - 1
            };
        }
    }

    fn current_image_mut(&mut self) -> Option<&mut ImageSlot> {
        self.image_cache
            .get_mut(self.current_index)
            .and_then(|slot| slot.as_mut())
    }

    fn current_quote(&self) -> Option<&AnimeQuote> {
        self.quotes.get(self.current_index)
    }

    fn instructions_line(&self) -> Line<'static> {
        let key_style = Style::default().fg(self.palette.instructions).bold();
        Line::from(vec![
            Span::raw(" Previous "),
            Span::styled("<Left>", key_style),
            Span::raw(" Next "),
            Span::styled("<Right>", key_style),
            Span::raw(" Quit "),
            Span::styled("<Q>", key_style),
            Span::raw(" "),
        ])
    }
}
