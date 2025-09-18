use std::fs;
use std::io;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use image::{GenericImageView, imageops::FilterType};
use ratatui::{
    DefaultTerminal, Frame,
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style, Stylize},
    symbols::border,
    text::{Line, Span, Text},
    widgets::{Block, Paragraph, Widget},
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct AnimeQuote {
    japanese: String,
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
    gradient: Vec<char>,
    detail_x: u32,
    detail_y: u32,
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

#[derive(Clone, Debug)]
struct AsciiPixel {
    ch: char,
    color: Color,
}

#[derive(Clone, Debug)]
struct AsciiArt {
    pixels: Vec<Vec<AsciiPixel>>,
}

impl AsciiArt {
    fn from_image_path(path: &str, settings: &AsciiSettings) -> Result<Self, image::ImageError> {
        let image = image::open(path)?;
        let (width, height) = image.dimensions();
        let base_width = settings.base_width.max(1);
        let detail_x = settings.detail_x.max(1);
        let detail_y = settings.detail_y.max(1);
        let aspect_ratio = height as f32 / width as f32;

        let sample_width = base_width * detail_x;
        let base_height = ((aspect_ratio * base_width as f32) * settings.char_aspect).max(1.0);
        let sample_height = (base_height * detail_y as f32).max(1.0).round() as u32;

        let resized =
            image.resize_exact(sample_width, sample_height.max(1), FilterType::CatmullRom);

        let rgba = resized.to_rgba8();
        let (width, height) = rgba.dimensions();
        let width_usize = width as usize;
        let height_usize = height as usize;

        let mut luminance = vec![vec![0.0; width_usize]; height_usize];
        let mut colors = vec![vec![(0u8, 0u8, 0u8, 0u8); width_usize]; height_usize];

        for y in 0..height_usize {
            for x in 0..width_usize {
                let pixel = rgba.get_pixel(x as u32, y as u32);
                let r = pixel[0];
                let g = pixel[1];
                let b = pixel[2];
                let a = pixel[3];
                colors[y][x] = (r, g, b, a);
                luminance[y][x] = if a < 32 {
                    1.0
                } else {
                    (0.2126 * (r as f32) + 0.7152 * (g as f32) + 0.0722 * (b as f32)) / 255.0
                };
            }
        }

        let mut buffer = luminance.clone();
        let mut pixels = vec![
            vec![
                AsciiPixel {
                    ch: ' ',
                    color: Color::Rgb(0, 0, 0),
                };
                width_usize
            ];
            height_usize
        ];

        let levels = settings.gradient.len().max(1);
        let max_index = (levels.saturating_sub(1)) as f32;

        for y in 0..height_usize {
            if y % 2 == 0 {
                for x in 0..width_usize {
                    Self::apply_dither_pixel(
                        x,
                        y,
                        true,
                        &mut buffer,
                        &colors,
                        &mut pixels,
                        &settings.gradient,
                        max_index,
                    );
                }
            } else {
                for x in (0..width_usize).rev() {
                    Self::apply_dither_pixel(
                        x,
                        y,
                        false,
                        &mut buffer,
                        &colors,
                        &mut pixels,
                        &settings.gradient,
                        max_index,
                    );
                }
            }
        }

        Ok(Self { pixels })
    }

    fn apply_dither_pixel(
        x: usize,
        y: usize,
        left_to_right: bool,
        buffer: &mut Vec<Vec<f32>>,
        colors: &[Vec<(u8, u8, u8, u8)>],
        pixels: &mut Vec<Vec<AsciiPixel>>,
        gradient: &[char],
        max_index: f32,
    ) {
        let (r, g, b, a) = colors[y][x];
        if a < 32 {
            pixels[y][x] = AsciiPixel {
                ch: ' ',
                color: Color::Rgb(r, g, b),
            };
            return;
        }

        let old = buffer[y][x].clamp(0.0, 1.0);
        let mut index = 0usize;
        let mut error = 0.0;

        if max_index > 0.0 {
            let scaled = old * max_index;
            let clamped = scaled.clamp(0.0, max_index);
            index = clamped.round() as usize;
            let new_value = (index as f32) / max_index;
            error = old - new_value;
        }

        let safe_index = index.min(gradient.len().saturating_sub(1));
        let ch = gradient.get(safe_index).copied().unwrap_or(' ');
        pixels[y][x] = AsciiPixel {
            ch,
            color: Color::Rgb(r, g, b),
        };

        if max_index <= 0.0 {
            return;
        }

        let width = buffer[0].len();
        let height = buffer.len();

        let mut add_error = |nx: isize, ny: isize, weight: f32| {
            if nx >= 0 && ny >= 0 && (nx as usize) < width && (ny as usize) < height {
                let cell = &mut buffer[ny as usize][nx as usize];
                *cell = (*cell + error * weight).clamp(0.0, 1.0);
            }
        };

        if left_to_right {
            add_error(x as isize + 1, y as isize, 7.0 / 16.0);
            add_error(x as isize - 1, y as isize + 1, 3.0 / 16.0);
            add_error(x as isize, y as isize + 1, 5.0 / 16.0);
            add_error(x as isize + 1, y as isize + 1, 1.0 / 16.0);
        } else {
            add_error(x as isize - 1, y as isize, 7.0 / 16.0);
            add_error(x as isize + 1, y as isize + 1, 3.0 / 16.0);
            add_error(x as isize, y as isize + 1, 5.0 / 16.0);
            add_error(x as isize - 1, y as isize + 1, 1.0 / 16.0);
        }
    }

    fn lines(&self) -> Vec<Line<'static>> {
        self.pixels
            .iter()
            .map(|row| {
                let spans: Vec<Span<'static>> = row
                    .iter()
                    .map(|pixel| {
                        Span::styled(pixel.ch.to_string(), Style::default().fg(pixel.color))
                    })
                    .collect();
                Line::from(spans)
            })
            .collect()
    }
}

fn main() -> io::Result<()> {
    let mut terminal = ratatui::init();
    let app_result = App::default().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug)]
pub struct App {
    quotes: Vec<AnimeQuote>,
    ascii_cache: Vec<Option<AsciiArt>>,
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
        let ascii_cache = quotes
            .iter()
            .map(|quote| {
                quote.image.as_deref().and_then(|path| {
                    match AsciiArt::from_image_path(path, &ascii_settings) {
                        Ok(art) => Some(art),
                        Err(error) => {
                            eprintln!("failed to load ascii art from {path}: {error}");
                            None
                        }
                    }
                })
            })
            .collect();
        Self {
            quotes,
            ascii_cache,
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

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
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

    fn current_ascii_art(&self) -> Option<&AsciiArt> {
        self.ascii_cache
            .get(self.current_index)
            .and_then(|art| art.as_ref())
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

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = Line::from(" Anime Quotes ".bold());
        let mut block = Block::bordered()
            .title(title.centered())
            .border_set(border::THICK);
        if self.show_instructions {
            block = block.title_bottom(self.instructions_line().centered());
        }

        let mut lines: Vec<Line> = Vec::new();

        if let Some(ascii) = self.current_ascii_art() {
            lines.extend(ascii.lines());
            lines.push(Line::from(""));
        }

        if let Some(quote) = self.current_quote() {
            let anime_style = Style::default().fg(self.palette.anime).bold();
            let character_style = Style::default().fg(self.palette.character).bold();
            let japanese_style = Style::default().fg(self.palette.japanese).bold();
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

        Paragraph::new(Text::from(lines))
            .centered()
            .block(block)
            .render(area, buf);
    }
}
