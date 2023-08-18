use core::fmt;
use lazy_static::lazy_static;
use spin;
use volatile::Volatile;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
struct ColorCode(u8);

impl ColorCode {
    fn new(text_color: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (text_color as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_char: u8,
    color: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH]; BUFFER_HEIGHT],
}

pub struct Writer {
    row: usize,
    col: usize,
    text_color: Color,
    background: Color,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn write_byte(&mut self, byte: u8) {
        self.write_colored_byte(byte, self.text_color, self.background)
    }

    pub fn write_colored_byte(&mut self, byte: u8, text_color: Color, background: Color) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.col >= BUFFER_WIDTH {
                    self.new_line();
                }

                self.buffer.chars[self.row][self.col].write(ScreenChar {
                    ascii_char: byte,
                    color: ColorCode::new(text_color, background),
                });
                self.col += 1;
            }
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                // anything else is not part of printable ASCII range
                _ => {
                    // make the text color red if it is not valid ascii, 
                    // but if the background is red, make the text yellow
                    if self.background == Color::Red {
                        self.write_colored_byte(0xfe, Color::Yellow, self.background);
                    } else {
                        self.write_colored_byte(0xfe, Color::Red, self.background);
                    }
                }
            }
        }
    }

    #[allow(dead_code)]
    pub fn write_colored_string(&mut self, s: &str, text_color: Color, background: Color) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII byte or newline
                0x20..=0x7e | b'\n' => self.write_colored_byte(byte, text_color, background),
                // not part of printable ASCII range
                _ => self.write_colored_byte(byte, text_color, background),
            }
        }
    }

    fn new_line(&mut self) {
        if self.row == BUFFER_HEIGHT - 1 {
            for row in 1..BUFFER_HEIGHT {
                for col in 0..BUFFER_WIDTH {
                    let character = self.buffer.chars[row][col].read();
                    self.buffer.chars[row - 1][col].write(character);
                }
            }
            self.clear_row(BUFFER_HEIGHT - 1);
            // when scrolling, down, the writer remains on the bottom row
        } else {
            self.row += 1;
        }
        self.col = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_char: b' ',
            color: ColorCode::new(self.text_color, self.background),
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

lazy_static! {
    pub static ref WRITER: spin::Mutex<Writer> = spin::Mutex::new(Writer {
        row: 0,
        col: 0,
        text_color: Color::White,
        background: Color::Black,
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    WRITER.lock().write_fmt(args).unwrap();
}

#[test_case]
fn test_println_simple() {
    println!("test_println_simple output");
}

#[test_case]
fn test_println_many() {
    for _ in 0..200 {
        println!("test_println_many output");
    }
}

#[test_case]
fn test_println_output() {
    // Fill the buffer so that the buffer is full
    for _ in 0..200 {
        println!("test_println_many output");
    }
    let s = "Some test string that fits on a single line";
    println!("{}", s);
    for (i, c) in s.chars().enumerate() {
        let screen_char = WRITER.lock().buffer.chars[BUFFER_HEIGHT - 2][i].read();
        assert_eq!(char::from(screen_char.ascii_char), c);
    }
}