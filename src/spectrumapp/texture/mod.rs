use std::collections::VecDeque;
use std::fmt;

pub struct KRGBAImage {
    pub dx: u32,
    pub dy: u32,
    pub pixels: Vec<u8>,
}
impl KRGBAImage {
    pub fn sub<'a>(&'a self) -> RGBASub<'a> {
        RGBASub {
            offset: 0,
            img: self,
            min: KPoint { x: 0, y: 0 },
            max: KPoint {
                x: self.dx,
                y: self.dy,
            },
        }
    }
}
pub struct RGBASub<'a> {
    pub offset: usize,
    pub min: KPoint,
    pub max: KPoint,
    pub img: &'a KRGBAImage,
}
impl<'a> RGBASub<'a> {
    pub fn sub_image<'b>(&self, min: KPoint, max: KPoint) -> Self {
        let i = self.pix_offset(&min);
        Self {
            offset: i,
            min,
            max,
            img: self.img,
        }
    }

    pub fn stride(&self) -> usize {
        self.img.dx as usize * 4
    }

    pub fn pix_offset(&self, p: &KPoint) -> usize {
        self.offset
            + (p.y as usize - self.min.y as usize) * self.stride()
            + (p.x as usize - self.min.x as usize) * 4
    }
}
pub struct KTexture {
    pub img: Option<KRGBAImage>,
    pub used_rect: KRect,
    pub allocated_rect: KRect,
}
#[derive(Clone, Eq, PartialEq)]
pub struct KPoint {
    pub x: u32,
    pub y: u32,
}

impl fmt::Debug for KPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KPoint{{{}, {}}}", self.x, self.y)
    }
}

// Integer region, usually square for allocation for texture
#[derive(Clone, Eq, PartialEq)]
pub struct KRect {
    pub min: KPoint,
    pub max: KPoint,
    pub atlas_id: u32,
}

impl fmt::Debug for KRect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KRect{{min:{:?}, max:{:?}, {}}}",
            self.min, self.max, self.atlas_id
        )
    }
}

impl KRect {
    pub fn at_origin(dx: u32, dy: u32, atlas_id: u32) -> Self {
        KRect {
            min: KPoint { x: 0, y: 0 },
            max: KPoint { x: dx, y: dy },
            atlas_id,
        }
    }

    pub fn dx(&self) -> u32 {
        self.max.x - self.min.x
    }
    pub fn dy(&self) -> u32 {
        self.max.y - self.min.y
    }

    pub fn scaled(&self, atlas_size: u32) -> FRect {
        let size_xy = atlas_size as f32;
        self.scaled_xy(size_xy, size_xy)
    }
    pub fn scaled_xy(&self, atlas_dx: f32, atlas_dy: f32) -> FRect {
        let scale_dx = self.dx() as f32 / atlas_dx;
        let scale_dy = self.dy() as f32 / atlas_dy;

        FRect {
            scale: FPoint {
                x: scale_dx,
                y: scale_dy,
            },
            min: FPoint {
                x: self.min.x as f32 / atlas_dx,
                y: self.min.y as f32 / atlas_dy,
            },
            max: FPoint {
                x: self.max.x as f32 / atlas_dx,
                y: self.max.y as f32 / atlas_dy,
            },
            atlas_id: self.atlas_id,
        }
    }

    pub fn split(&self) -> [KRect; 4] {
        let minx = self.min.x;
        let miny = self.min.y;

        let maxx = self.max.x;
        let maxy = self.max.y;

        let cenx = (minx + maxx) / 2;
        let ceny = (miny + maxy) / 2;

        [
            KRect {
                min: KPoint { x: minx, y: miny },
                max: KPoint { x: cenx, y: ceny },
                atlas_id: self.atlas_id,
            },
            KRect {
                min: KPoint { x: cenx, y: miny },
                max: KPoint { x: maxx, y: ceny },
                atlas_id: self.atlas_id,
            },
            KRect {
                min: KPoint { x: minx, y: ceny },
                max: KPoint { x: cenx, y: maxy },
                atlas_id: self.atlas_id,
            },
            KRect {
                min: KPoint { x: cenx, y: ceny },
                max: KPoint { x: maxx, y: maxy },
                atlas_id: self.atlas_id,
            },
        ]
    }
}
#[derive(Default, Debug)]
pub struct FPoint {
    pub x: f32,
    pub y: f32,
}
// scaled version of KRect for texture rendering purposes

#[derive(Debug)]
pub struct FRect {
    pub scale: FPoint,
    pub min: FPoint,
    pub max: FPoint,
    pub atlas_id: u32,
}

impl Default for FRect {
    fn default() -> Self {
        Self {
            scale: FPoint { x: 1.0, y: 1.0 },
            min: FPoint { x: 0.0, y: 0.0 },
            max: FPoint { x: 128.0, y: 128.0 },
            atlas_id: 0,
        }
    }
}

pub struct RectAllocator {
    squares_by_size: [VecDeque<KRect>; 16],
}
// ffs
fn power2_of(mut x: u32) -> u32 {
    let mut y = 0;
    while x > 1 {
        y += 1;
        x = x >> 1;
    }
    y
}
impl RectAllocator {
    pub fn new() -> RectAllocator {
        RectAllocator {
            squares_by_size: Default::default(),
        }
    }

    pub fn with_size_deque<F>(&mut self, size: u32, cb: F)
    where
        F: FnOnce(&mut VecDeque<KRect>),
    {
        let exp = power2_of(size);

        let q = &mut self.squares_by_size[exp as usize];

        cb(q);
    }

    pub fn provide(&mut self, rect: KRect) {
        let dx = rect.dx();
        //let dy = tex.subRect.dy();

        self.with_size_deque(dx as u32, |q| {
            q.push_back(rect);
        });
    }

    pub fn provide_atlases(&mut self, atlas_size: u32, num_atlases: u32) {
        for i in 0..num_atlases {
            self.provide(KRect {
                min: KPoint { x: 0, y: 0 },
                max: KPoint {
                    x: atlas_size,
                    y: atlas_size,
                },
                atlas_id: i,
            })
        }
    }

    pub fn allocate(&mut self, size: u32) -> Option<KRect> {
        //let dy = tex.subRect.dy();
        let exp = power2_of(size);
        {
            let q = &mut self.squares_by_size[exp as usize];

            match q.pop_front() {
                None => {}
                Some(rect) => {
                    return Some(rect);
                }
            }
        }
        let mut nexp = exp + 1;
        // no free rects of that size found, search for larger ones
        loop {
            let up_q = &mut self.squares_by_size[nexp as usize];

            match up_q.pop_front() {
                None => {}
                Some(mut rect) => {
                    while nexp > exp {
                        nexp -= 1;
                        let [a, b, c, d] = rect.split();
                        let down_q = &mut self.squares_by_size[nexp as usize];

                        rect = a;
                        //down_q.push_back(a);
                        down_q.push_back(b);
                        down_q.push_back(c);
                        down_q.push_back(d);
                    }
                    return Some(rect);
                }
            }

            nexp += 1;

            if nexp >= 16 {
                return None;
            }
        }
    }
}

#[test]
pub fn test_rect_alloc() {
    let mut ra = RectAllocator::new();

    ra.provide(KRect {
        min: KPoint { x: 0, y: 0 },
        max: KPoint { x: 1024, y: 1024 },
        atlas_id: 0,
    });
    {
        let a = ra.allocate(32);
        assert!(a != None);
        assert_eq!(
            a.unwrap(),
            KRect {
                min: KPoint { x: 0, y: 0 },
                max: KPoint { x: 32, y: 32 },
                atlas_id: 0,
            }
        );
    }
    {
        let b = ra.allocate(32);
        assert!(b != None);
        let br = b.unwrap();

        assert_eq!(br.dx(), 32);
        assert_eq!(br.dy(), 32);
    }
    {
        let c = ra.allocate(32);
        assert!(c != None);
        let cr = c.unwrap();

        assert_eq!(cr.dx(), 32);
        assert_eq!(cr.dy(), 32);
    }
}
