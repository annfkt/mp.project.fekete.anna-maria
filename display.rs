use core::result;

use embedded_hal::blocking::i2c::{Write, WriteRead};
use embedded_hal_1::delay::DelayNs;
use mcp23017::{Error, PinMode, MCP23017};

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum Mcp23017Pin {
    A0 = 0,
    A1 = 1,
    A2 = 2,
    A3 = 3,
    A4 = 4,
    A5 = 5,
    A6 = 6,
    A7 = 7,
    B0 = 8,
    B1 = 9,
    B2 = 10,
    B3 = 11,
    B4 = 12,
    B5 = 13,
    B6 = 14,
    B7 = 15,
}

pub const ADDR: u8 = 0x20; // Default I2C address

// MCP23017 registers
pub const IODIRA: u8 = 0x00; // I/O direction register for Port A
pub const IODIRB: u8 = 0x01; // I/O direction register for Port B
pub const GPIOA: u8 = 0x12; // GPIO register for Port A
pub const GPIOB: u8 = 0x13; // GPIO register for Port B

// MCP23017 pin mappings based on Python library
pub const LCD_RS: Mcp23017Pin = Mcp23017Pin::B7; // Register Select (RS)
pub const LCD_E: Mcp23017Pin = Mcp23017Pin::B5; // Enable (E)
pub const LCD_D4: Mcp23017Pin = Mcp23017Pin::B4;
pub const LCD_D5: Mcp23017Pin = Mcp23017Pin::B3;
pub const LCD_D6: Mcp23017Pin = Mcp23017Pin::B2;
pub const LCD_D7: Mcp23017Pin = Mcp23017Pin::B1;
pub const LCD_RW: Mcp23017Pin = Mcp23017Pin::B6; // Read/Write (RW)

// MCP23017Pin pins for RGB LED
pub const RGB_RED: Mcp23017Pin = Mcp23017Pin::A6;
pub const RGB_GREEN: Mcp23017Pin = Mcp23017Pin::A7;
pub const RGB_BLUE: Mcp23017Pin = Mcp23017Pin::B0;
pub const LCD_BACKLIGHT: Mcp23017Pin = Mcp23017Pin::A5;

// MCP23017Pin pins for Buttons
pub const BTN_LEFT: Mcp23017Pin = Mcp23017Pin::A4;
pub const BTN_UP: Mcp23017Pin = Mcp23017Pin::A3;
pub const BTN_DOWN: Mcp23017Pin = Mcp23017Pin::A2;
pub const BTN_RIGHT: Mcp23017Pin = Mcp23017Pin::A1;
pub const BTN_SELECT: Mcp23017Pin = Mcp23017Pin::A0;

// LCD commands
pub const LCD_CLEARDISPLAY: u8 = 0x01;
pub const LCD_RETURNHOME: u8 = 0x02;
pub const LCD_ENTRYMODESET: u8 = 0x04;
pub const LCD_DISPLAYCONTROL: u8 = 0x08;
pub const LCD_CURSORSHIFT: u8 = 0x10;
pub const LCD_FUNCTIONSET: u8 = 0x20;
pub const LCD_SETCGRAMADDR: u8 = 0x40;
pub const LCD_SETDDRAMADDR: u8 = 0x80;

// Entry flags
pub const LCD_ENTRYLEFT: u8 = 0x02;
pub const LCD_ENTRYSHIFTDECREMENT: u8 = 0x00;

// Control flags
pub const LCD_DISPLAYON: u8 = 0x04;
pub const LCD_CURSORON: u8 = 0x02;
pub const LCD_CURSOROFF: u8 = 0x00;
pub const LCD_BLINKON: u8 = 0x01;
pub const LCD_BLINKOFF: u8 = 0x00;

// Move flags
pub const LCD_DISPLAYMOVE: u8 = 0x08;
pub const LCD_MOVERIGHT: u8 = 0x04;
pub const LCD_MOVELEFT: u8 = 0x00;

// Function set flags
pub const LCD_4BITMODE: u8 = 0x00;
pub const LCD_2LINE: u8 = 0x08;
pub const LCD_1LINE: u8 = 0x00;
pub const LCD_5X8DOTS: u8 = 0x00;

// Direction constants
pub const LEFT_TO_RIGHT: usize = 0;
pub const RIGHT_TO_LEFT: usize = 1;

// Row offset addresses for different LCD lines
pub const LCD_ROW_OFFSETS: [u8; 4] = [0x00, 0x40, 0x14, 0x54];

// Custom error type
#[derive(Debug)]
pub enum LcdError {
    I2c,
    Mcp,
    Other,
}

impl<E> From<Error<E>> for LcdError {
    fn from(_value: Error<E>) -> Self {
        LcdError::Mcp
    }
}

impl From<embassy_rp::i2c::Error> for LcdError {
    fn from(_value: embassy_rp::i2c::Error) -> Self {
        LcdError::I2c
    }
}

pub struct CharLCDRGBI2C<I2C: Write + WriteRead, D: DelayNs> {
    mcp: MCP23017<I2C>,
    delay: D,
    columns: usize,
    lines: usize,
    backlight: bool,    // Backlight status
    rgb: [Mcp23017Pin; 3], // RGB pins
    color_value: [u8; 3],
    display_control: u8,
    display_mode: u8,
    display_function: u8,
    row: usize,
    column: usize,
    column_align: bool,
    direction: usize,
}

impl<E, I2C: Write<Error=E> + WriteRead<Error = E>, D: DelayNs> CharLCDRGBI2C<I2C, D> where LcdError: From<E> {
    pub fn new(i2c: I2C, delay: D, columns: usize, lines: usize) -> Result<Self, LcdError> {
        // Use map_err for the MCP error conversion
        let mcp = MCP23017::default(i2c)?;

        let mut lcd = CharLCDRGBI2C {
            mcp,
            delay,
            columns,
            lines,
            backlight: true,
            rgb: [RGB_RED, RGB_GREEN, RGB_BLUE],
            color_value: [0, 0, 0],
            display_control: 0,
            display_mode: 0,
            display_function: 0,
            row: 0,
            column: 0,
            column_align: false,
            direction: 0, // Assuming 0 for LEFT_TO_RIGHT
        };

        lcd.setup_pins()?;
        lcd.initialize()?;

        Ok(lcd)
    }

    fn setup_pins(&mut self) -> Result<(), LcdError> {
        // Set LCD control pins as outputs
        for pin in [LCD_RS, LCD_E, LCD_D4, LCD_D5, LCD_D6, LCD_D7, LCD_RW] {
            self.mcp.pin_mode(pin as u8, PinMode::OUTPUT)?;
        }

        // Set RGB LED pins as outputs
        for pin in [RGB_RED, RGB_GREEN, RGB_BLUE] {
            self.mcp.pin_mode(pin as u8, PinMode::OUTPUT)?;
        }

        // Set Button pins as inputs with pull-up
        for pin in [BTN_LEFT, BTN_UP, BTN_DOWN, BTN_RIGHT, BTN_SELECT] {
            self.mcp.pin_mode(pin as u8, PinMode::INPUT)?;
            self.mcp.pull_up(pin as u8, true)?;
        }

        Ok(())
    }

    fn initialize(&mut self) -> Result<(), LcdError> {
        // Wait for LCD to be ready
        self.delay.delay_ms(50);

        // Pull RS low to begin commands
        self.mcp.digital_write(LCD_RS as u8, false)?;
        self.mcp.digital_write(LCD_E as u8 as u8, false)?;
        self.mcp.digital_write(LCD_RW as u8, false)?;

        // 4-bit mode initialization sequence
        self.write4bits(0x03)?;
        self.delay.delay_ms(5);
        self.write4bits(0x03)?;
        self.delay.delay_ms(5);
        self.write4bits(0x03)?;
        self.delay.delay_ms(1);
        self.write4bits(0x02)?; // Set to 4-bit mode
        self.delay.delay_ms(1);

        // Initialize display control
        self.display_control = LCD_DISPLAYON | LCD_CURSOROFF | LCD_BLINKOFF;
        self.display_function = LCD_4BITMODE | LCD_1LINE | LCD_2LINE | LCD_5X8DOTS;
        self.display_mode = LCD_ENTRYLEFT | LCD_ENTRYSHIFTDECREMENT;

        // Write to display control
        self.write_command(LCD_DISPLAYCONTROL | self.display_control)?;
        // Write to display function
        self.write_command(LCD_FUNCTIONSET | self.display_function)?;
        // Set entry mode
        self.write_command(LCD_ENTRYMODESET | self.display_mode)?;

        // Clear display
        self.clear()?;

        // Initialize tracking variables
        self.row = 0;
        self.column = 0;
        self.column_align = false;
        self.direction = LEFT_TO_RIGHT;

        // Turn off all RGB LEDs initially
        self.set_color(0, 0, 0)?;

        Ok(())
    }

    fn write4bits(&mut self, value: u8) -> Result<(), LcdError> {
        // Set data pins
        self.mcp.digital_write(
            LCD_D4 as u8,
            value & 0x01 > 0 
        )?;
        self.mcp.digital_write(
            LCD_D5 as u8,
            value & 0x02 > 0 
        )?;
        self.mcp.digital_write(
            LCD_D6 as u8,
            value & 0x04 > 0 
        )?;
        self.mcp.digital_write(
            LCD_D7 as u8,
            value & 0x08 > 0 
        )?;

        // Pulse the enable pin
        self.pulse_enable()?;

        Ok(())
    }

    fn pulse_enable(&mut self) -> Result<(), LcdError> {
        self.mcp.digital_write(LCD_E as u8, false)?;
        self.delay.delay_us(1);
        self.mcp.digital_write(LCD_E as u8, true)?;
        self.delay.delay_us(1);
        self.mcp.digital_write(LCD_E as u8, false)?;
        self.delay.delay_us(100);
        Ok(())
    }

    fn write8(&mut self, value: u8, char_mode: bool) -> Result<(), LcdError> {
        // Set the RS pin based on char_mode
        self.mcp
            .digital_write(LCD_RS as u8, char_mode )?;

        // Send upper 4 bits
        self.write4bits(value >> 4)?;
        // Send lower 4 bits
        self.write4bits(value & 0x0F)?;
        Ok(())
    }

    fn write_command(&mut self, value: u8) -> Result<(), LcdError> {
        self.write8(value, false)?;
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), LcdError> {
        self.write_command(LCD_CLEARDISPLAY)?;
        self.delay.delay_ms(3);
        self.row = 0;
        self.column = 0;
        Ok(())
    }

    pub fn home(&mut self) -> Result<(), LcdError> {
        self.write_command(LCD_RETURNHOME)?;
        self.delay.delay_ms(3);
        self.row = 0;
        self.column = 0;
        Ok(())
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8) -> Result<(), LcdError> {
        // Any value > 1 turns LED on (inverse of Python logic)
        // LOW = on for common anode RGB LED
        self.mcp
            .digital_write(self.rgb[0] as u8, if r > 1 { false } else { true })?; // R
        self.mcp
            .digital_write(self.rgb[1] as u8, if g > 1 { false } else { true })?; // G
        self.mcp
            .digital_write(self.rgb[2] as u8, if b > 1 { false } else { true })?; // B

        self.color_value = [r, g, b];
        Ok(())
    }

    pub fn set_cursor(&mut self, col: usize, row: usize) -> Result<(), LcdError> {
        let row_offsets = [0x00, 0x40, 0x14, 0x54]; // For 16x2 or 20x4 LCD

        if row >= self.lines {
            return Err(LcdError::Other);
        }

        let command = LCD_SETDDRAMADDR | (col as u8 + row_offsets[row]);
        self.write_command(command)?;

        self.row = row;
        self.column = col;
        Ok(())
    }

    pub fn set_backlight(&mut self, on: bool) -> Result<(), LcdError> {
        if on {
            self.mcp.pin_mode(LCD_BACKLIGHT as u8, PinMode::OUTPUT)?;
            self.backlight = true;
            // println!("Backlight ON")
        } else {
            self.mcp.pin_mode(LCD_BACKLIGHT as u8, PinMode::INPUT)?;
            self.backlight = false;
            // println!("Backlight OFF")
        }
        Ok(())
    }

    pub fn cursor_position(&mut self, mut column: usize, mut row: usize) -> Result<(), LcdError> {
        if row >= self.lines {
            row = self.lines - 1;
        }
        if column >= self.columns {
            column = self.columns - 1;
        }
        self.write_command(LCD_SETDDRAMADDR | (column as u8 + LCD_ROW_OFFSETS[row]))?;
        self.row = row;
        self.column = column;
        Ok(())
    }

    pub fn message(&mut self, message: &str) -> Result<(), LcdError> {

        let mut line = self.row;
        let mut initial_character = 0;

        for char in message.chars() {
            if initial_character == 0 {
                let col;
                if self.display_mode & LCD_ENTRYLEFT > 0 {
                    col = self.column;
                } else {
                    col = self.columns - 1 - self.column;
                }
                self.cursor_position(col, line)?;
                initial_character += 1;
            }

            if char == '\n' {
                line += 1;
                let col;
                if self.display_mode & LCD_ENTRYLEFT > 0 {
                    if self.column_align {
                        col = self.column;
                    } else {
                        col = 0;
                    }
                } else {
                    if self.column_align {
                        col = self.column;
                    } else {
                        col = self.columns - 1;
                    }
                }
                self.cursor_position(col, line)?;
            } else {
                self.write8(char as u8, true)?;
            }
        }

        self.column = 0;
        self.row = 0;

        Ok(())
    }

    pub fn read_button_left(&mut self) -> Result<bool, LcdError> {
        Ok(self.mcp.digital_read(BTN_LEFT as u8)?)
    }
    
    pub fn read_button_right(&mut self) -> Result<bool,LcdError> {
        Ok(self.mcp.digital_read(BTN_RIGHT as u8)?)
    }

    pub fn read_button_up(&mut self) -> Result<bool,LcdError> {
        Ok(self.mcp.digital_read(BTN_UP as u8)?)
    }

    pub fn read_button_down(&mut self) -> Result<bool,LcdError> {
        Ok(self.mcp.digital_read(BTN_DOWN as u8)?)
    }

    pub fn read_button_select(&mut self) -> Result<bool, LcdError> {
        Ok(self.mcp.digital_read(BTN_SELECT as u8)?)
    }
}