use imgui::{ImGui, Ui, im_str};

enum DataType {
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    Float,
    Double,
}

impl Default for DataType {
    fn default() -> Self {
        DataType::S32
    }
}

impl DataType {
    fn get_size(&self) -> usize {
        match self {
            DataType::S8 => 1,
            DataType::U8 => 1,
            DataType::S16 => 2,
            DataType::U16 => 2,
            DataType::S32 => 4,
            DataType::U32 => 4,
            DataType::S64 => 8,
            DataType::U64 => 8,
            DataType::Float => 4,
            DataType::Double => 8,
        }
    }
}

enum DataFormat {
    Bin,
    Dec,
    Hex,
}

#[derive(Default)]
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

pub struct MemoryEditor {
    open: bool, // set to false when DrawWindow() was closed. ignore if not using DrawWindow().
    read_only: bool, // disable any editing.
    cols: usize, // number of columns to display.
    opt_show_options: bool, // display options button/context menu. when disabled, options will be locked unless you provide your own UI for them.
    opt_show_data_preview: bool, // display a footer previewing the decimal/binary/hex/float representation of the currently selected bytes.
    opt_show_hexII: bool, // display values in HexII representation instead of regular hexadecimal: hide null/zero bytes, ascii values as ".X".
    opt_show_ascii: bool, // display ASCII representation on the right side.
    opt_grey_out_zeroes: bool, // display null/zero bytes using the TextDisabled color.
    opt_uppercase_hex: bool, // display hexadecimal values as "FF" instead of "ff".
    opt_mid_cols_count: usize, // set to 0 to disable extra spacing between every mid-cols.
    opt_addr_digits_count: usize, // number of addr digits to display (default calculated based on maximum displayed addr).
    highlight_color: u32, // background color of highlighted bytes.

    // [Internal state]
    contents_width_changed: bool,
    data_preview_addr: usize,
    data_editing_addr: usize,
    data_editing_take_focus: bool,
    data_input_buf: [u8; 32],
    addr_input_buf: [u8; 32],
    goto_addr: usize,
    highlight_min: usize,
    highlight_max: usize,
    preview_endianess: i32,
    preview_data_type: DataType
}

impl Default for MemoryEditor {
    fn default() -> Self {
        MemoryEditor {
            open: true,
            read_only: false,
            cols: 16,
            opt_show_options: true,
            opt_show_data_preview: false,
            opt_show_hexII: false,
            opt_show_ascii: true,
            opt_grey_out_zeroes: true,
            opt_uppercase_hex: true,
            opt_mid_cols_count: 8,
            opt_addr_digits_count: 0,
            highlight_color: 0xFF_FF_FF_32,

            contents_width_changed: false,
            data_preview_addr: usize::max_value(),
            data_editing_addr: usize::max_value(),
            data_editing_take_focus: false,
            data_input_buf: [0; 32],
            addr_input_buf: [0; 32],
            goto_addr: usize::max_value(),
            highlight_min: usize::max_value(),
            highlight_max: usize::max_value(),
            preview_endianess: 0,
            preview_data_type: DataType::S32,
        }
    }
}

impl MemoryEditor {
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
        sizes.window_width = sizes.pos_ascii_end + style.scrollbar_size + style.window_padding.x * 2.0 + sizes.glyph_width;
    }

    pub fn draw_contents(&mut self, ui: &Ui) {
        let mem_data = &[0u8, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let mem_size = mem_data.len();
        let base_display_addr = 0x0000;
        let mut sizes = Sizes::default();
        
        self.calc_sizes(ui, &mut sizes, mem_size, base_display_addr);
        let style = ui.imgui().style();
        // We begin into our scrolling region with the 'ImGuiWindowFlags_NoMove' in order to prevent click from moving the window.
        // This is used as a facility since our main click detection code doesn't assign an ActiveId so the click would normally be caught as a window-move.

        let height_separator = style.item_spacing.y;
        let mut footer_height = 0.0;

        let todo_fixme_get_frame_height_with_spacing = 1.0;

        if self.opt_show_options {
            footer_height += height_separator + todo_fixme_get_frame_height_with_spacing;
        }
        if self.opt_show_data_preview {
            footer_height += height_separator + todo_fixme_get_frame_height_with_spacing + todo_fixme_get_frame_height_with_spacing * 3.0;
        }
        unsafe {
            assert!(imgui::sys::igBeginChild(
                im_str!("##scrolling").as_ptr(),
                (0.0, -footer_height).into(),
                false,
                imgui::sys::ImGuiWindowFlags::NoMove,
            ));
        }
        let draw_list = unsafe {
            imgui::sys::igGetWindowDrawList()
        };
        unsafe {
            imgui::sys::igPushStyleVarVec2(
                imgui::sys::ImGuiStyleVar::FramePadding,
                (0.0, 0.0).into(),
            );
        }
        unsafe {
            imgui::sys::igPushStyleVarVec2(
                imgui::sys::ImGuiStyleVar::ItemSpacing,
                (0.0, 0.0).into(),
            )
        }

        let line_total_count = mem_size + self.cols - 1 / self.cols;
        let mut clipper = imgui::sys::ImGuiListClipper {
            start_pos_y: 0.0,
            items_height: 0.0,
            items_count: 0,
            step_no: 0,
            display_start: 0,
            display_end: 0,
        };
        unsafe {
            imgui::sys::ImGuiListClipper_Begin(
                &mut clipper as *mut imgui::sys::ImGuiListClipper,
                line_total_count as i32,
                sizes.line_height,
            );
        }
        let visible_start_addr = clipper.display_start as usize + self.cols;
        let visible_end_addr = clipper.display_end as usize + self.cols;

        let mut data_next = false;

        if self.read_only || self.data_editing_addr >= mem_size {
            self.data_editing_addr = usize::max_value();
        }
        if self.data_preview_addr >= mem_size {
            self.data_preview_addr = usize::max_value();
        }

        let preview_data_type_size = if self.opt_show_data_preview {
            self.preview_data_type.get_size()
        } else {
            0
        };

        let mut data_editing_addr_backup = self.data_editing_addr;
        let mut data_editing_addr_next = usize::max_value();
        if self.data_editing_addr != usize::max_value() {
            // Move cursor but only apply on next frame so scrolling will be synchronized
            // Because currently we can't change the scrolling while the window is being rendered
            if unsafe {imgui::sys::igIsKeyPressed(
                imgui::sys::igGetKeyIndex(imgui::sys::ImGuiKey::UpArrow),
                false,
            )} && self.data_editing_addr >= self.cols {
                data_editing_addr_next = self.data_editing_addr - self.cols;
                self.data_editing_take_focus = true;
            } else if unsafe {imgui::sys::igIsKeyPressed(
                imgui::sys::igGetKeyIndex(imgui::sys::ImGuiKey::DownArrow),
                false
            )} && self.data_editing_addr < mem_size - self.cols {
                data_editing_addr_next = self.data_editing_addr + self.cols;
                self.data_editing_take_focus = true;
            } else if unsafe {imgui::sys::igIsKeyPressed(
                imgui::sys::igGetKeyIndex(imgui::sys::ImGuiKey::LeftArrow),
                false
            )} && self.data_editing_addr > 0 {
                data_editing_addr_next = self.data_editing_addr - 1;
                self.data_editing_take_focus = true;
            } else if unsafe {imgui::sys::igIsKeyPressed(
                imgui::sys::igGetKeyIndex(imgui::sys::ImGuiKey::RightArrow),
                false
            )} && self.data_editing_addr < mem_size - 1 {
                data_editing_addr_next = self.data_editing_addr + 1;
                self.data_editing_take_focus = true;
            }
        }

        if data_editing_addr_next != usize::max_value() && (data_editing_addr_next / self.cols) != (data_editing_addr_backup / self.cols) {
            // Track cursor movements
            let scroll_offset = (data_editing_addr_next / self.cols) - (data_editing_addr_backup / self.cols);
            let scroll_desired = (scroll_offset < 0 && data_editing_addr_next < visible_start_addr + self.cols * 2) || (scroll_offset > 0 && data_editing_addr_next > visible_end_addr - self.cols * 2);
            if scroll_desired {
                unsafe {
                    imgui::sys::igSetScrollY(
                        imgui::sys::igGetScrollY() + scroll_offset as f32 * sizes.line_height
                    );
                }
            }
        }

        //Draw vertical separator
        let window_pos = ui.get_window_pos();
        if self.opt_show_ascii {
            unsafe {
                imgui::sys::ImDrawList_AddLine(
                    draw_list as *mut imgui::sys::ImDrawList,
                    (window_pos.0 + sizes.pos_ascii_start - sizes.glyph_width, window_pos.1).into(),
                    (window_pos.0 + sizes.pos_ascii_start - sizes.glyph_width, window_pos.1 + 9999.0).into(),
                    imgui::sys::igGetColorU32U32(imgui::sys::ImGuiCol::Border as u32),
                    0.0
                );
            }
        }

        let color_text = unsafe {
            imgui::sys::igGetColorU32U32(imgui::sys::ImGuiCol::Text as u32)
        };
        let color_disabled = if self.opt_grey_out_zeroes {
            unsafe {
                imgui::sys::igGetColorU32U32(imgui::sys::ImGuiCol::TextDisabled as u32)
            }
        } else {
               color_text
        };

        let format_address = if self.opt_uppercase_hex {
            "string to format uppercase hex"
        } else {
            "string to format lowercase hex"
        };

        let format_data = if self.opt_uppercase_hex {
            "string to format uppercase hex"
        } else {
            "string to format lowercase hex"
        };

        let format_range = if self.opt_uppercase_hex {
            "string to format uppercase hex"
        } else {
            "string to format lowercase hex"
        };

        let format_byte = if self.opt_uppercase_hex {
            "string to format uppercase hex"
        } else {
            "string to format lowercase hex"
        };

        let format_byte_space = if self.opt_uppercase_hex {
            "string to format uppercase hex"
        } else {
            "string to format lowercase hex"
        };

        let mut line_i = clipper.display_start;
        while line_i < clipper.display_end {
            line_i += 1;
            let mut addr = line_i as usize * self.cols;
            ui.text(im_str!("{:x}: {}", sizes.addr_digits_count, base_display_addr + addr as usize));
            
            // draw hexadecimal
            let mut n = 0;
            while n < self.cols && addr < mem_size {
                n += 1;
                addr += 1;

                let mut byte_pos_x = sizes.pos_hex_start + sizes.hex_cell_width * n as f32;
                if self.opt_mid_cols_count > 0 {
                    byte_pos_x += (n / self.opt_mid_cols_count) as f32 * sizes.spacing_between_mid_cols;
                }
                ui.same_line(byte_pos_x);

                let is_highlight_from_user_range = (addr >= self.highlight_min && addr < self.highlight_max);
                let is_highlight_from_user_func = false; // TODO
                let is_highlight_from_preview = (addr >= self.data_preview_addr && addr < self.data_preview_addr + self.preview_data_type.get_size());

                if is_highlight_from_user_range || is_highlight_from_user_func || is_highlight_from_preview {
                    let pos = ui.get_cursor_screen_pos();
                    let mut highlight_width = sizes.glyph_width * 2.0;
                    let is_next_byte_highlighted = (addr + 1 < mem_size) && ((self.highlight_max != usize::max_value() && addr + 1 < self.highlight_max) || false); // TODO
                    if is_next_byte_highlighted || (n + 1) == self.cols {
                        highlight_width = sizes.hex_cell_width;
                        if self.opt_mid_cols_count > 0 && n > 0 && (n + 1) < self.cols && ((n + 1) % self.opt_mid_cols_count) == 0 {
                            highlight_width += sizes.spacing_between_mid_cols;
                        }
                        unsafe {
                            imgui::sys::ImDrawList_AddRectFilled(
                                draw_list as *mut imgui::sys::ImDrawList,
                                (pos.0 + highlight_width, pos.1 + sizes.line_height).into(),
                                (pos.0 + highlight_width, pos.1 + sizes.line_height).into(), //TODO FIXME wrong parameters?
                                self.highlight_color,
                                0.0,
                                imgui::sys::ImDrawCornerFlags::empty(),
                            )
                        }
                    }

                    if self.data_editing_addr == addr {
                        let mut data_write = false;
                        unsafe {
                            imgui::sys::igPushIDInt(addr as i32);
                        }
                        if self.data_editing_take_focus {
                            unsafe {
                                imgui::sys::igSetKeyboardFocusHere(0);
                            }
                            unsafe {
                                imgui::sys::igCaptureKeyboardFromApp(true);
                            }
                            self.addr_input_buf = {
                                let mut buf = [0; 32];
                                let vec: Vec<u8> = format!("{}: {}", sizes.addr_digits_count, base_display_addr + addr)
                                    .bytes()
                                    .take(buf.len())
                                    .collect();
                                buf.copy_from_slice(&vec); // TODO ReadFn
                                buf
                            };
                            self.data_input_buf = {
                                let mut buf = [0; 32];
                                let vec: Vec<u8> = format!("{}", mem_data[addr])
                                    .bytes()
                                    .take(buf.len())
                                    .collect();
                                buf.copy_from_slice(&vec); // TODO ReadFn
                                buf
                            };
                        }
                        unsafe {
                            imgui::sys::igPushItemWidth(sizes.glyph_width * 2.0);
                        }
                        // TODO InputTextCallback
                        unsafe {
                            if ui.input_text(
                                im_str!("##data"),
                            std::mem::transmute(&mut self.data_input_buf)
                            ).build() {
                                data_write = true;
                                data_next = true;
                            } else if !self.data_editing_take_focus && !imgui::sys::igIsItemActive() {
                                self.data_editing_addr = usize::max_value();
                                data_editing_addr_next = usize::max_value();
                            }
                        }
                        self.data_editing_take_focus = false;
                        unsafe {
                            imgui::sys::igPopItemWidth();
                            if true { // Todo callback
                                data_write = true;
                                data_next = true;
                            }
                            if data_editing_addr_next != usize::max_value() {
                                data_write = false;
                                data_next = false;
                            }
                            let mut data_input_value = 0;
                            // todo scanf
                        }
                        unsafe { imgui::sys::igPopID(); }
                    }
                    else {
                        let b = mem_data[addr];

                        if self.opt_show_hexII {
                            if b >= 32 && b < 128 {
                                ui.text(im_str!("{}", b));
                            } else if b == 0xFF && self.opt_grey_out_zeroes {
                                ui.text_disabled(im_str!("## "));
                            } else if b == 0x00 {
                                ui.text(im_str!("  "));
                            } else {
                                ui.text(im_str!("{}", b));
                            }
                        } else {
                            if b == 0 && self.opt_grey_out_zeroes {
                                ui.text_disabled(im_str!("00 "));
                            } else {
                                ui.text(im_str!("{}", b));
                            }
                        }

                        unsafe {
                            if !self.read_only && ui.is_item_hovered() && imgui::sys::igIsMouseClicked(0, false) {
                                self.data_editing_take_focus = true;
                                data_editing_addr_next = addr;
                            }
                        }
                    }
                }
            }

            if self.opt_show_ascii {
                // Draw ASCII values
                unsafe {
                    imgui::sys::igSameLine(sizes.pos_ascii_start, 0.0);
                }
                let mut pos = ui.get_cursor_screen_pos();
                addr = line_i as usize * self.cols;
                unsafe {
                    imgui::sys::igPushIDInt(line_i);
                }
                if ui.invisible_button(
                    im_str!("ascii"),
                    (sizes.pos_ascii_end - sizes.pos_ascii_start, sizes.line_height)
                ) {
                    let thing = unsafe {
                        ((*imgui::sys::igGetIO()).mouse_pos.x - pos.0 / sizes.glyph_width) as usize
                    };
                    self.data_editing_addr = addr + thing;
                    self.data_editing_take_focus = true;
                }
                unsafe {
                    imgui::sys::igPopID();
                }
                let mut n = 0;
                while n < self.cols && addr < mem_size - 1 {
                    n += 1;
                    addr += 1;
                    if addr == self.data_editing_addr {
                        unsafe {
                            imgui::sys::ImDrawList_AddRectFilled(
                                draw_list as *mut imgui::sys::ImDrawList,
                                pos.into(),
                                (pos.0 + sizes.glyph_width, pos.1 + sizes.line_height).into(), //TODO FIXME wrong parameters?
                                imgui::sys::igGetColorU32U32(imgui::sys::ImGuiCol::FrameBg as u32),
                                0.0,
                                imgui::sys::ImDrawCornerFlags::empty(),
                            );
                            imgui::sys::ImDrawList_AddRectFilled(
                                draw_list as *mut imgui::sys::ImDrawList,
                                pos.into(),
                                (pos.0 + sizes.glyph_width, pos.1 + sizes.line_height).into(), //TODO FIXME wrong parameters?
                                imgui::sys::igGetColorU32U32(imgui::sys::ImGuiCol::TextSelectedBg as u32),
                                0.0,
                                imgui::sys::ImDrawCornerFlags::empty(),
                            );
                        }
                    }
                    let c = mem_data[addr]; // TODO readfn
                    let display_c = if c < 32 || c >= 128 { '.' } else { c as char }.to_string();
                    let color_for_this = if &display_c == "." { color_disabled } else { color_text };
                    unsafe {
                        imgui::sys::ImDrawList_AddText(
                            draw_list as *mut imgui::sys::ImDrawList,
                            pos.into(),
                            color_for_this,
                            display_c.as_ptr() as *const i8,
                            display_c.as_ptr().offset(1) as *const i8,
                        )
                    }
                    pos.0 += sizes.glyph_width;
                }
            }
        }
        unsafe {
            imgui::sys::ImGuiListClipper_End(
                &mut clipper as *mut imgui::sys::ImGuiListClipper
            );
        }
        unsafe {
            imgui::sys::igPopStyleVar(2);
            imgui::sys::igEndChild();
        }

        if data_next && self.data_editing_addr < mem_size {
            self.data_editing_addr += 1;
            self.data_preview_addr = self.data_editing_addr;
            self.data_editing_take_focus = true;
        } else if data_editing_addr_next != usize::max_value() {
            self.data_editing_addr = data_editing_addr_next;
            self.data_preview_addr = self.data_editing_addr;
        }

        let mut next_show_data_preview = self.opt_show_data_preview;
        if self.opt_show_options {
            ui.separator();

            // Options menu
            if ui.button(im_str!("Options"), (20.0, 20.0)) {
                ui.open_popup(im_str!("context"));
            }
            ui.popup_modal(im_str!("context")).build(|| {
                ui.push_item_width(56.0);
                let mut my_cols = self.cols as i32;
                if ui.drag_int(im_str!("##cols"), &mut my_cols).build() {
                    self.contents_width_changed = true;
                }
                ui.pop_item_width();
                ui.checkbox(im_str!("Show data preview"), &mut next_show_data_preview);
                ui.checkbox(im_str!("Show HexII"), &mut self.opt_show_hexII);
                if ui.checkbox(im_str!("Show ASCII"), &mut self.opt_show_ascii) {
                    self.contents_width_changed = true;
                }
                ui.checkbox(im_str!("Grey out zeroes"), &mut self.opt_grey_out_zeroes);
                ui.checkbox(im_str!("Uppercase Hex"), &mut self.opt_uppercase_hex);
                self.cols = my_cols as usize;
            });
            ui.same_line(0.0);
            ui.text(im_str!("{}: {}, {}, {}", sizes.addr_digits_count, base_display_addr, sizes.addr_digits_count, base_display_addr + mem_size - 1));
            ui.same_line(0.0);
            ui.push_item_width((sizes.addr_digits_count as f32 + 1.0) * sizes.glyph_width + style.frame_padding.x * 2.0);

            unsafe {
                if ui.input_text(
                    im_str!("##addr"),
                    std::mem::transmute(&mut self.addr_input_buf)
                ).build() {
                    self.goto_addr = base_display_addr + 1; // todo wtf
                    self.highlight_min = usize::max_value();
                    self.highlight_max = usize::max_value();
                }
            }
            ui.pop_item_width();

            if self.goto_addr != usize::max_value() {
                if self.goto_addr < mem_size {
                    unsafe {
                        imgui::sys::igBeginChild(
                            im_str!("##scrolling").as_ptr(),
                            (0.0, 0.0).into(),
                            false,
                            imgui::sys::ImGuiWindowFlags::empty()
                        );
                        imgui::sys::igSetScrollFromPosY(
                            imgui::sys::igGetCursorStartPos().y + (self.goto_addr / self.cols) as f32 * imgui::sys::igGetTextLineHeight(),
                            0.0,
                        );
                        imgui::sys::igEndChild();
                    }
                    self.data_editing_addr = self.goto_addr;
                    self.data_preview_addr = self.goto_addr;
                    self.data_editing_take_focus = true;
                } else {
                    self.goto_addr = usize::max_value();
                }
            }
        }

        if self.opt_show_data_preview {
            ui.separator();
            unsafe {
                imgui::sys::igAlignTextToFramePadding();
            }
            ui.text(im_str!("Preview as: "));
            ui.same_line(0.0);
            ui.push_item_width(sizes.glyph_width * 10.0 + style.frame_padding.x * 2.0 + style.item_inner_spacing.x);
        }
    }
}