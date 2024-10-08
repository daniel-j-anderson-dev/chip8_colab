mod instructions;

use crate::nibbles::{combine_three_nibbles, combine_two_nibbles, get_first_nibble, get_second_nibble};

pub struct Chip8 {
    memory: [u8; 4096],

    /// Index to the current byte in memory.
    program_counter: u16,

    /// Often called `I`.
    /// Also called memory index register.
    address_register: u16,

    /// General purpose registers often called `VX` where X is the index.
    /// The last byte (`VF`) is also used as a flag for carries or other purposes
    variable_register: [u8; 16],

    /// Keeps track of return memory locations when a subroutine is called
    call_stack: [u16; 16],

    /// Keeps track of which entry in the stack should be returned to.
    /// Determines current position in stack.
    call_stack_index: usize,

    /// Decrements at 60hz until zero
    delay_timer: u8,

    /// Decrements at 60hz until zero when a sound is played
    sound_timer: u8,

    /// `false` represents a black pixel. `true` represents a white pixel
    display: [[bool; 64]; 32],

    /// A collection of four rows. `true` represents a pressed button. `false` represents a unpressed button
    /// ```text
    ///   0   1   2   3
    /// ╔═══╦═══╦═══╦═══╗
    /// ║ 1 ║ 2 ║ 3 ║ C ║ 0
    /// ╠═══╬═══╬═══╬═══╣
    /// ║ 4 ║ 5 ║ 6 ║ D ║ 1
    /// ╠═══╬═══╬═══╬═══╣
    /// ║ 7 ║ 8 ║ 9 ║ E ║ 2
    /// ╠═══╬═══╬═══╬═══╣
    /// ║ A ║ 0 ║ B ║ F ║ 3
    /// ╚═══╩═══╩═══╩═══╝
    /// ```
    keypad: [[bool; 4]; 4],
}

impl Chip8 {
    /// Offset is commonly done because of old standards.
    /// Most programs written for Chip8 expect programs to start here.
    pub const PROGRAM_MEMORY_OFFSET: u16 = 200;

    pub fn new() -> Chip8 {
        Self {
            memory: [0; 4096],
            program_counter: Self::PROGRAM_MEMORY_OFFSET,
            address_register: 0,
            variable_register: [0; 16],
            call_stack: [0; 16],
            call_stack_index: 0,
            delay_timer: 0,
            sound_timer: 0,
            display: [[false; 64]; 32],
            keypad: [[false; 4]; 4],
        }
    }

    /// Returns an array contain the four nibbles of an opcode.
    /// (a nibble is a four bit number or single hexadecimal digit)
    ///
    /// TODO: Add bounds checking
    fn get_current_instruction(&self) -> [u8; 4] {
        let program_counter = self.program_counter as usize;

        let most_significant_byte = self.memory[program_counter];
        let least_significant_byte = self.memory[program_counter + 1];

        [
            get_first_nibble(most_significant_byte),
            get_second_nibble(most_significant_byte),
            get_first_nibble(least_significant_byte),
            get_second_nibble(least_significant_byte),
        ]
    }

    fn execute_current_instruction(&mut self) {
        let nibbles = self.get_current_instruction();

        let address = combine_three_nibbles(nibbles[1], nibbles[2], nibbles[3]);
        let value = combine_two_nibbles(nibbles[2], nibbles[3]);
        let x_register_index = nibbles[1] as usize;
        let y_register_index = nibbles[2] as usize;
        let sprite_height = nibbles[3];

        match nibbles {
            [0x0, _, _, _] => {},
            [0x0, 0x0, 0xE, 0x0] => self.clear_screen(),
            [0x0, 0x0, 0xE, 0xE] => self.return_subroutine(),
            [0x1, _, _, _] => self.jump(address),
            [0x2, _, _, _] => self.call_subroutine(address),
            [0x3, _, _, _] => self.skip_if_equal_value(x_register_index, value),
            [0x4, _, _, _] => self.skip_if_equal_value(x_register_index, value),
            [0x5, _, _, 0x0] => self.skip_if_equal(x_register_index, y_register_index),
            [0x6, _, _, _] => self.assign_value(x_register_index, value),
            [0x7, _, _, _] => self.add_assign_value(x_register_index, value),
            [0x8, _, _, 0x0] => self.assign(x_register_index, y_register_index),
            [0x8, _, _, 0x1] => self.bitwise_or(x_register_index, y_register_index),
            [0x8, _, _, 0x2] => self.bitwise_and(x_register_index, y_register_index),
            [0x8, _, _, 0x3] => self.bitwise_xor(x_register_index, y_register_index),
            [0x8, _, _, 0x4] => self.add_assign(x_register_index, y_register_index),
            [0x8, _, _, 0x5] => self.sub_assign(x_register_index, y_register_index),
            [0x8, _, _, 0x6] => self.right_shift_assign(x_register_index, y_register_index),
            [0x8, _, _, 0x7] => self.sub_assign_swapped(x_register_index, y_register_index),
            [0x8, _, _, 0xE] => self.left_shift_assign(x_register_index, y_register_index),
            [0x9, _, _, 0x0] => self.skip_if_not_equal(x_register_index, y_register_index),
            [0xA, _, _, _] => self.set_address_register(address),
            [0xB, _, _, _] => self.jump_offset(address),
            [0xC, _, _, _] => self.random_number_assign(x_register_index, value),
            [0xD, _, _, _] => self.draw_sprite(x_register_index, y_register_index, sprite_height),
            [0xE, _, 0x9, 0xE] => self.skip_on_key_pressed(x_register_index),
            [0xE, _, 0xA, 0x1] => self.skip_on_key_not_pressed(x_register_index),
            [0xF, _, 0x0, 0x7] => self.store_delay_timer(x_register_index),
            [0xF, _, 0x0, 0xA] => self.wait_for_key_press(x_register_index),
            [0xF, _, 0x1, 0x5] => self.set_delay_timer(x_register_index),
            [0xF, _, 0x1, 0x8] => self.set_sound_timer(x_register_index),
            [0xF, _, 0x1, 0xE] => self.address_register_add_assign(x_register_index),
            [0xF, _, 0x2, 0x9] => self.set_address_register_to_character_address(x_register_index),
            [0xF, _, 0x3, 0x3] => self.store_binary_coded_decimal_at_address_register(x_register_index),
            [0xF, _, 0x5, 0x5] => self.store_variable_registers(x_register_index),
            [0xF, _, 0x6, 0x5] => self.load_variable_registers(x_register_index),
            _ => {},
        }

        unimplemented!();
    }
}
