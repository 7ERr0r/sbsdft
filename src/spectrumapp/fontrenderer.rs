use super::texture::FRect;
use super::texture::KPoint;
use super::texture::KRGBAImage;
use super::texture::KRect;
use super::texture::RGBASub;
use std::rc::Rc;

use crate::klog;

pub trait FontVertex {
    fn pos_col_tex(pos: [f32; 2], color: u32, tex: [f32; 2]) -> Self;
}

// fn filled_vec(val: i32, mut len: usize) -> Vec<i32> {
//     let mut v = Vec::with_capacity(len);
//     unsafe {
//         v.set_len(len);
//         let p: *mut i32 = v.as_mut_ptr();

//         while len > 0 {
//             *(p.add(len)) = val;
//             len -= 1;
//         }
//     }
//     v
// }

pub struct FontAtlas {
    tex_rect: FRect,
    char_width: Vec<f32>,
    color_code: Vec<u32>,
    char_id_map: Vec<i32>,
    debug_char_id_order: Vec<char>,
}
impl FontAtlas {
    pub fn new() -> Self {
        let mut fa = Self {
            tex_rect: FRect::default(),
            char_width: vec![6.0; 512],
            color_code: vec![0; 32],
            char_id_map: vec![-1; 65536],
            debug_char_id_order: Vec::new(),
        };
        fa.make_color_code();
        fa.create_char_id_map();

        fa
    }
    pub fn resize_texture(&mut self, krect: &KRect, atlas_size: u32, img: &KRGBAImage) {
        self.tex_rect = krect.scaled(atlas_size);
        klog!("tex_rext: {:?}", self.tex_rect);
        self.read_default_font_texture(img, 16, 16);
    }

    fn create_char_id_map(&mut self) {
        let mut char_id_order = String::new();

        char_id_order.push_str("\u{00c0}\u{00c1}\u{00c2}\u{00c8}\u{00ca}\u{00cb}\u{00cd}\u{00d3}\u{00d4}\u{00d5}\u{00da}\u{00df}\u{00e3}\u{00f5}\u{011f}\u{0130}\u{0131}\u{0152}\u{0153}\u{015e}\u{015f}\u{0174}\u{0175}\u{017e}\u{0207}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000}\u{0000} !\"#$%&\'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~\u{0000}\u{00c7}\u{00fc}\u{00e9}\u{00e2}\u{00e4}\u{00e0}\u{00e5}\u{00e7}\u{00ea}\u{00eb}\u{00e8}\u{00ef}\u{00ee}\u{00ec}\u{00c4}\u{00c5}\u{00c9}\u{00e6}\u{00c6}\u{00f4}\u{00f6}\u{00f2}\u{00fb}\u{00f9}\u{00ff}\u{00d6}\u{00dc}\u{00f8}\u{00a3}\u{00d8}\u{00d7}\u{0192}\u{00e1}\u{00ed}\u{00f3}\u{00fa}\u{00f1}\u{00d1}\u{00aa}\u{00ba}\u{00bf}\u{00ae}\u{00ac}\u{00bd}\u{00bc}\u{00a1}\u{00ab}\u{00bb}\u{2591}\u{2592}\u{2593}\u{2502}\u{2524}\u{2561}\u{2562}\u{2556}\u{2555}\u{2563}\u{2551}\u{2557}\u{255d}\u{255c}\u{255b}\u{2510}\u{2514}\u{2534}\u{252c}\u{251c}\u{2500}\u{253c}\u{255e}\u{255f}\u{255a}\u{2554}\u{2569}\u{2566}\u{2560}\u{2550}\u{256c}\u{2567}\u{2568}\u{2564}\u{2565}\u{2559}\u{2558}\u{2552}\u{2553}\u{256b}\u{256a}\u{2518}\u{250c}\u{2588}\u{2584}\u{258c}\u{2590}\u{2580}\u{03b1}\u{03b2}\u{0393}\u{03c0}\u{03a3}\u{03c3}\u{03bc}\u{03c4}\u{03a6}\u{0398}\u{03a9}\u{03b4}\u{221e}\u{2205}\u{2208}\u{2229}\u{2261}\u{00b1}\u{2265}\u{2264}\u{2320}\u{2321}\u{00f7}\u{2248}\u{00b0}\u{2219}\u{00b7}\u{221a}\u{207f}\u{00b2}\u{25a0}\u{0000}");

        let char_id_map: &mut [i32] = &mut self.char_id_map;

        for (i, c) in char_id_order.chars().enumerate() {
            let c32 = c as i32;
            if c32 >= 0 && c32 < char_id_map.len() as i32 {
                //log(&format!("addding {}", c32).to_string());
                char_id_map[c32 as usize] = i as i32;
            }
        }
        self.debug_char_id_order = char_id_order.chars().collect();
    }

    fn make_color_code(&mut self) {
        for index in 0..32_usize {
            let i = index as u32;
            let j: u32 = (i >> 3 & 1) * 85;
            let mut b: u32 = (i >> 2 & 1) * 170 + j;
            let mut g: u32 = (i >> 1 & 1) * 170 + j;
            let mut r: u32 = (i >> 0 & 1) * 170 + j;

            if i == 6 {
                b += 85;
            }

            if i >= 16 {
                b /= 4;
                g /= 4;
                r /= 4;
            }
            //log(&format!("color code {}", index).to_string());

            self.color_code[index] = (b & 255) << 16 | (g & 255) << 8 | r & 255;
        }
    }
    fn read_default_font_texture(&mut self, img: &KRGBAImage, gw: u32, gh: u32) {
        let sw = img.dx / gw;
        let sh = img.dy / gh;

        let sub = img.sub();
        for char_id in 0..256 {
            let cx = (char_id & 0xF) * sw;
            let cy = (char_id >> 4) * sh;

            let width = self.read_glyph(&sub.sub_image(
                KPoint { x: cx, y: cy },
                KPoint {
                    x: cx + sw,
                    y: cy + sh,
                },
            ));
            //console_log!("char id{}={} width = {}", char_id, self.debug_char_id_order.get(char_id as usize).unwrap(), width);
            self.char_width[char_id as usize] = 2.0 + (width / (img.dx as f32 / 128.0)).ceil();
        }
    }
    fn read_glyph(&mut self, img: &RGBASub) -> f32 {
        let mut startx: i32 = 0;
        let mut endx: i32 = 0;
        let mut start = true;

        'xloop: for x in img.min.x..img.max.x {
            for y in img.min.y..img.max.y {
                let offset = img.pix_offset(&KPoint { x, y });
                let a = img.img.pixels[offset + 3];
                let filled = a != 0;

                if start {
                    if filled {
                        startx = x as i32;
                        endx = x as i32;
                        start = false;
                        continue 'xloop;
                    }
                } else {
                    if filled {
                        endx = x as i32;
                        continue 'xloop;
                    }
                }
            }
        }
        (endx - startx) as f32
    }
}

//#[allow(dead_code)]
pub struct FontRenderer<'a, V>
where
    V: FontVertex,
{
    vertices: &'a mut Vec<V>,
    font_atlas: Rc<FontAtlas>,
    scale_factor: f32,
    pub ui_scale: f32,

    unicode_flag: bool,
    _pos_atlas: i32,
    pos_x: f32,
    pos_y: f32,
    random_style: bool,
    bold_style: bool,
    strikethrough_style: bool,
    underline_style: bool,
    italic_style: bool,
    cached_color: u32,
    alpha: f32,

    red: f32,
    blue: f32,
    green: f32,
}

impl<'a, V> FontRenderer<'a, V>
where
    V: FontVertex,
{
    pub fn new(font_atlas: Rc<FontAtlas>, vertices: &'a mut Vec<V>) -> Self {
        let font_renderer = Self {
            vertices,
            font_atlas,
            scale_factor: 1.0,
            ui_scale: 1.0,

            unicode_flag: false,
            _pos_atlas: 0,
            pos_x: 1.0,
            pos_y: 1.0,
            random_style: false,
            bold_style: false,
            strikethrough_style: false,
            underline_style: false,
            italic_style: false,
            cached_color: 0xFFFFFFFF,
            alpha: 1.0,
            red: 1.0,
            blue: 1.0,
            green: 1.0,
        };

        //font_renderer.reset();

        font_renderer
    }

    pub fn reset(&mut self) {
        self.pos_x = 1.0;
        self.pos_y = 1.0;
        self.reset_styles();
        self.cached_color = 0xFFFFFFFF;
        self.alpha = 1.0;
    }
    pub fn reset_styles(&mut self) {
        self.random_style = false;
        self.bold_style = false;
        self.strikethrough_style = false;
        self.underline_style = false;
        self.italic_style = false;
    }
    pub fn get_char_id(&self, c: char) -> i32 {
        let c32 = c as i32;
        let char_id_map = &self.font_atlas.char_id_map;
        if c32 >= 0 && c32 < char_id_map.len() as i32 {
            return char_id_map[c32 as usize];
        } else {
            return -1;
        }
    }
    pub fn add_char(&mut self, c: char, italic: bool) -> f32 {
        if c == ' ' {
            5.0
        } else {
            let char_id = self.get_char_id(c);
            if char_id != -1 && !self.unicode_flag {
                self.add_default_char(char_id, italic)
            } else {
                5.0
            }
        }
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    pub fn add_default_char(&mut self, char_id: i32, italic: bool) -> f32 {
        let char_size;
        let chari;
        let charj;
        let char_width;
        let char_height;
        let mut char_offsety = 0.0;

        let char_scale_x = 1.0 / (128.0);
        let char_scale_y = 1.0 / (128.0);

        let atlas_id = char_id >> 8;
        //println!("atlas_id: {}", atlas_id);
        // console.log(charId)
        let correction = 0.2;
        if atlas_id == 0 {
            char_size = self.font_atlas.char_width[char_id as usize];
            chari = (char_id & 0x0F) as f32 * 8.0;
            charj = (char_id >> 4) as f32 * 8.0;
            char_width = 8.0 - correction;
            char_height = 8.0 - correction;
        } else {
            char_size = 8.0; // self.charWidthAccented[c];
            chari = 128.0 + ((char_id & 0x0F) as f32 * 9.0);
            charj = ((char_id & 0xFF) >> 4) as f32 * 12.0;
            char_width = 9.0 - correction;
            char_height = 12.0 - correction;

            char_offsety = -3.0;
        }
        let char_skew = if italic { 1.0 } else { 0.0 };

        let color = self.cached_color;


        let x = self.pos_x;
        let y = self.pos_y;

        let a = [x + char_skew, y+char_offsety];
        let at = [chari * char_scale_x, charj * char_scale_y];

        let b = [x-char_skew, y+char_height + char_offsety];
        let bt = [chari * char_scale_x, (charj + char_height) * char_scale_y];

        let c = [x+char_width - 1.0 + char_skew, y+char_offsety];
        let ct = [(chari + char_width - 1.0) * char_scale_x, charj * char_scale_y];

        let d = [x+char_width - 1.0 - char_skew, y+char_height + char_offsety];
        let dt = [(chari + char_width - 1.0) * char_scale_x, (charj + char_height) * char_scale_y];


        let scale = self.ui_scale;
        let a = [a[0]*scale, a[1]*scale];
        let b = [b[0]*scale, b[1]*scale];
        let c = [c[0]*scale, c[1]*scale];
        let d = [d[0]*scale, d[1]*scale];

        let v = &mut self.vertices;
        v.push(V::pos_col_tex(a, color, at));
        v.push(V::pos_col_tex(b, color, bt));
        v.push(V::pos_col_tex(c, color, ct));

        v.push(V::pos_col_tex(d, color, dt));
        v.push(V::pos_col_tex(c, color, ct));
        v.push(V::pos_col_tex(b, color, bt));




        char_size
    }

    pub fn set_color(&mut self, r: u8, g: u8, b: u8, a: u8) {
        self.cached_color =
            ((a as u32) << 24) | ((b as u32) << 16) | ((g as u32) << 8) | (r as u32);
    }
    #[allow(dead_code)]
    #[warn(unused_assignments)]
    pub fn render_string_at_pos(&mut self, rendered_str: &str, drop_shadow: bool) {
        let mut paragraph_coded = false;

        for (_i, c) in rendered_str.chars().enumerate() {
            if c == 167 as char {
                paragraph_coded = true;
                continue;
            }
            if paragraph_coded {
                paragraph_coded = false;
                if c >= '\u{100}' {
                    continue;
                }
                // console.log("color")
                let col = c as u8;

                let num = col >= 48 && col <= 57; // 0 9
                let af = col >= 97 && col <= 102;
                if num || af {
                    let mut i1 = col;
                    if num {
                        i1 -= 48;
                    } // '0'
                    if af {
                        i1 -= 97 - 10;
                    } // 'a'-10

                    self.reset_styles();

                    if i1 > 15 {
                        i1 = 15;
                    }

                    if drop_shadow {
                        i1 += 16;
                    }

                    let rgba = self.font_atlas.color_code[i1 as usize];
                    // self._textColor = j1
                    self.set_color(
                        (rgba >> 16) as u8,
                        ((rgba >> 8) & 255) as u8,
                        (rgba & 255) as u8,
                        (self.alpha * 255.0) as u8,
                    );
                } else if col == 107 {
                    // k
                    self.random_style = true;
                } else if col == 108 {
                    // l
                    self.bold_style = true;
                } else if col == 109 {
                    // m
                    self.strikethrough_style = true;
                } else if col == 110 {
                    // n
                    self.underline_style = true;
                } else if col == 111 {
                    // o
                    self.italic_style = true;
                } else if col == 114 {
                    // r
                    self.reset_styles();
                    self.set_color(
                        (self.red * 255.0) as u8,
                        (self.blue * 255.0) as u8,
                        (self.green * 255.0) as u8,
                        (self.alpha * 255.0) as u8,
                    );
                }
            } else {
                let char_id = self.get_char_id(c);

                if self.random_style && char_id != -1 {}

                let f1 = if self.unicode_flag {
                    0.5
                } else {
                    1.0 / self.scale_factor
                };
                let flag = (c == 0 as char || char_id == -1 || self.unicode_flag) && drop_shadow;

                if flag {
                    self.pos_x -= f1;
                    self.pos_y -= f1;
                }
                // console.log("adding", c, self.posX, self.posY)
                let mut f = self.add_char(c, self.italic_style);

                if flag {
                    self.pos_x += f1;
                    self.pos_y += f1;
                }

                if self.bold_style {
                    self.pos_x += f1;

                    if flag {
                        self.pos_x -= f1;
                        self.pos_y -= f1;
                    }

                    self.add_char(c, self.italic_style);
                    self.pos_x -= f1;

                    if flag {
                        self.pos_x += f1;
                        self.pos_y += f1;
                    }

                    f += f1;
                }

                self.pos_x += f;
            }
        }
    }
    #[allow(dead_code)]
    pub fn render_string(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        mut color: u32,
        drop_shadow: bool,
    ) -> u32 {
        if text.len() == 0 {
            return 0;
        } else {
            if (color & 0xfc000000) == 0 {
                color |= 0xff000000;
            }

            if drop_shadow {
                color = (color & 0xfcfcfc) >> 2 | color & 0xff000000;
            }

            self.red = ((color >> 16 & 255) as f32) / 255.0;
            self.blue = ((color >> 8 & 255) as f32) / 255.0;
            self.green = ((color & 255) as f32) / 255.0;
            self.alpha = ((color >> 24 & 255) as f32) / 255.0;

            self.cached_color = color;
            self.pos_x = x;
            self.pos_y = y;
            self.scale_factor = 1.0;

            self.render_string_at_pos(text, drop_shadow);

            return self.pos_x as u32;
        }
    }
    #[allow(dead_code)]
    pub fn draw_string(
        &mut self,
        text: &str,
        x: f32,
        y: f32,
        color: u32,
        drop_shadow: bool,
    ) -> u32 {
        let mut len: u32;
        if drop_shadow {
            len = self.render_string(text, x + 1.0, y + 1.0, color, true);
            self.render_string(text, x + 0.5, y + 0.5, color, true); // second shadow

            let shadow_len = self.render_string(text, x, y, color, false);

            if shadow_len > len {
                len = shadow_len
            }
        } else {
            len = self.render_string(text, x, y, color, false);
        }

        return len;
    }
}
