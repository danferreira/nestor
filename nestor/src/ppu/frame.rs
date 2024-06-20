#[derive(Debug, Clone)]
pub struct Frame {
    width: usize,
    pub data: Vec<u8>,
}

impl Frame {
    pub fn new(width: usize, height: usize) -> Self {
        Frame {
            width,
            data: vec![0; width * height * 3],
        }
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, rgb: (u8, u8, u8)) {
        let base = y * 3 * self.width + x * 3;
        if base + 2 < self.data.len() {
            self.data[base] = rgb.0;
            self.data[base + 1] = rgb.1;
            self.data[base + 2] = rgb.2;
        }
    }

    pub fn to_rgba(&self) -> Vec<u8> {
        let mut buffer: Vec<u8> = vec![];
        for color in self.data.chunks_exact(3) {
            buffer.push(color[0]);
            buffer.push(color[1]);
            buffer.push(color[2]);
            buffer.push(255);
        }

        buffer
    }
}
