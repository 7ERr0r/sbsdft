use bytemuck::{Pod, Zeroable};

use super::fontrenderer::FontVertex;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PosColVertex {
    pub pos: [i16; 2],
    pub color: u32,
}

// #[repr(C)]
// #[derive(Clone, Copy, Pod, Zeroable)]
// struct PosColVertex {
//     pos: [f32; 2],
//     color: u32,
// }
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct PosColTexVertex {
    pub pos: [f32; 2],
    pub color: u32,
    pub tex: [f32; 2],
}

impl FontVertex for PosColTexVertex {
    fn pos_col_tex(pos: [f32; 2], color: u32, tex: [f32; 2]) -> Self {
        PosColTexVertex { pos, color, tex }
    }
}

pub struct DisplayMsSecond(pub f32);
impl<'a> std::fmt::Display for DisplayMsSecond {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0 < 1.0 {
            write!(f, "{:.1} ms", self.0 * 1000.0)?;
        } else {
            write!(f, "{:.1} s", self.0)?;
        }

        Ok(())
    }
}
