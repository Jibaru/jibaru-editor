//! `.cube` LUT loading (Adobe/Resolve format, 1D or 3D).

use crate::fx::AuxImage;
use crate::text::f32_to_f16;

#[derive(Debug, Clone, PartialEq)]
pub struct Lut3d {
    pub size: u32,
    /// RGB triplets, red varying fastest, then green, then blue.
    pub data: Vec<[f32; 3]>,
}

impl Lut3d {
    pub fn identity(size: u32) -> Self {
        let n = size as usize;
        let mut data = Vec::with_capacity(n * n * n);
        let s = (size - 1) as f32;
        for b in 0..n {
            for g in 0..n {
                for r in 0..n {
                    data.push([r as f32 / s, g as f32 / s, b as f32 / s]);
                }
            }
        }
        Self { size, data }
    }

    /// Parses a `.cube` file. 1D LUTs are expanded to a 33³ cube.
    pub fn parse_cube(text: &str) -> anyhow::Result<Self> {
        let mut size3 = 0usize;
        let mut size1 = 0usize;
        let mut dmin = [0f32; 3];
        let mut dmax = [1f32; 3];
        let mut rows: Vec<[f32; 3]> = Vec::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let mut it = line.split_whitespace();
            let head = it.next().unwrap_or("");
            let nums = |it: std::str::SplitWhitespace| -> anyhow::Result<[f32; 3]> {
                let v: Vec<f32> = it.map(|s| s.parse::<f32>()).collect::<Result<_, _>>()?;
                anyhow::ensure!(v.len() >= 3, "expected 3 values");
                Ok([v[0], v[1], v[2]])
            };
            match head {
                "TITLE" => {}
                "LUT_3D_SIZE" => size3 = it.next().unwrap_or("0").parse()?,
                "LUT_1D_SIZE" => size1 = it.next().unwrap_or("0").parse()?,
                "DOMAIN_MIN" => dmin = nums(it)?,
                "DOMAIN_MAX" => dmax = nums(it)?,
                "LUT_3D_INPUT_RANGE" | "LUT_1D_INPUT_RANGE" => {
                    let v: Vec<f32> = it.map(|s| s.parse::<f32>()).collect::<Result<_, _>>()?;
                    if v.len() == 2 {
                        dmin = [v[0]; 3];
                        dmax = [v[1]; 3];
                    }
                }
                _ if head.parse::<f32>().is_ok() => {
                    let v: Vec<f32> = line.split_whitespace().map(|s| s.parse::<f32>()).collect::<Result<_, _>>()?;
                    anyhow::ensure!(v.len() >= 3, "bad LUT row: {line}");
                    rows.push([v[0], v[1], v[2]]);
                }
                _ => {}
            }
        }
        let _ = (dmin, dmax);
        if size3 > 0 {
            anyhow::ensure!(rows.len() == size3 * size3 * size3, "expected {} rows, got {}", size3.pow(3), rows.len());
            anyhow::ensure!((2..=256).contains(&size3), "unsupported LUT size {size3}");
            return Ok(Self { size: size3 as u32, data: rows });
        }
        anyhow::ensure!(size1 >= 2 && rows.len() == size1, "invalid 1D LUT");
        let curve = |c: usize, x: f32| {
            let p = x * (size1 - 1) as f32;
            let i = (p.floor() as usize).min(size1 - 2);
            let k = p - i as f32;
            rows[i][c] * (1.0 - k) + rows[i + 1][c] * k
        };
        let mut lut = Self::identity(33);
        for v in &mut lut.data {
            *v = [curve(0, v[0]), curve(1, v[1]), curve(2, v[2])];
        }
        Ok(lut)
    }

    pub fn load(path: &str) -> anyhow::Result<Self> {
        Self::parse_cube(&std::fs::read_to_string(path)?)
    }

    pub fn to_aux(&self) -> AuxImage {
        let mut data = Vec::with_capacity(self.data.len() * 8);
        for v in &self.data {
            for c in [v[0], v[1], v[2], 1.0] {
                data.extend_from_slice(&f32_to_f16(c).to_le_bytes());
            }
        }
        AuxImage {
            width: self.size,
            height: self.size,
            depth: self.size,
            format: wgpu::TextureFormat::Rgba16Float,
            data,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_cube() {
        let mut s = String::from("TITLE \"t\"\nLUT_3D_SIZE 2\n");
        for b in 0..2 {
            for g in 0..2 {
                for r in 0..2 {
                    s += &format!("{} {} {}\n", r, g, b);
                }
            }
        }
        let l = Lut3d::parse_cube(&s).unwrap();
        assert_eq!(l, Lut3d::identity(2));
    }
}
