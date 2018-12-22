use imgui::{ImGui, Ui, im_str};

enum DataType {
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    Float,
    Double,
}

enum DataFormat {
    Bin,
    Dec,
    Hex,
}

struct Sizes {
    addr_digits_count: usize,
    line_height: f32,
    glyph_width: f32,
    hex_cell_width: f32,
    spacing_between_mid_cols: f32,
    pos_hex_start: f32,
    pos_hex_end: f32,
    pos_ascii_start: f32,
    pos_ascii_end: f32,
    window_width: f32,
}

#[derive(Default)]
pub struct MemoryEditor {
    cols: usize, // number of columns to display.
    opt_show_options: bool, // display options button/context menu. when disabled, options will be locked unless you provide your own UI for them.
    opt_show_data_preview: bool, // display a footer previewing the decimal/binary/hex/float representation of the currently selected bytes.
    opt_show_hexII: bool, // display values in HexII representation instead of regular hexadecimal: hide null/zero bytes, ascii values as ".X".
    opt_show_ascii: bool, // display ASCII representation on the right side.
    opt_grey_out_zeroes: bool, // display null/zero bytes using the TextDisabled color.
    opt_uppercase_hex: bool, // display hexadecimal values as "FF" instead of "ff".
    opt_mid_cols_count: usize, // set to 0 to disable extra spacing between every mid-cols.
    opt_addr_digits_count: usize, // number of addr digits to display (default calculated based on maximum displayed addr).
}

impl MemoryEditor {
    fn new() -> Self {
        MemoryEditor {
            cols: 16,
            opt_show_options: true,
            opt_show_data_preview: false,
            opt_show_hexII: false,
            opt_show_ascii: true,
            opt_grey_out_zeroes: true,
            opt_uppercase_hex: true,
            opt_mid_cols_count: 8,
            opt_addr_digits_count: 0,
        }
    }

    fn calc_sizes(&self, ui: &Ui, sizes: &mut Sizes, mem_size: usize, base_display_addr: usize) {
        let style = ui.imgui().style();
        sizes.addr_digits_count = self.opt_addr_digits_count;
        if sizes.addr_digits_count == 0 {
            let mut n = base_display_addr + mem_size - 1;
            while n > 0 {
                n >>= 4;
                sizes.addr_digits_count += 1;
            }
        }
        sizes.line_height = ui.get_text_line_height_with_spacing();
        sizes.glyph_width = ui.calc_text_size(im_str!("F"), false, 0.0).x + 1.0; // We assume the font is mono-space
        sizes.hex_cell_width = (sizes.glyph_width * 2.5).round(); // "FF " we include trailing space in the width to easily catch clicks everywhere
        sizes.spacing_between_mid_cols = (sizes.hex_cell_width * 0.25).round(); // Every opt_mid_cols_count we add a bit of extra spacing
        sizes.pos_hex_start = (sizes.addr_digits_count + 2) as f32 * sizes.glyph_width;
        sizes.pos_hex_end = sizes.pos_hex_start + (sizes.hex_cell_width * self.cols as f32);
        sizes.pos_ascii_start = sizes.pos_hex_end;
        sizes.pos_ascii_end = sizes.pos_hex_end;
        if self.opt_show_ascii {
            sizes.pos_ascii_start = sizes.pos_hex_end + sizes.glyph_width;
            if self.opt_mid_cols_count > 0 {
                sizes.pos_ascii_start += ((self.cols + self.opt_mid_cols_count - 1) / self.opt_mid_cols_count) as f32 * sizes.spacing_between_mid_cols;
            }
            sizes.pos_ascii_end = sizes.pos_ascii_start + self.cols as f32 * sizes.glyph_width;
        }
    }

    pub fn draw_contents(&self, ui: &Ui) {
        let mem_data = &[0u8];
    }
}