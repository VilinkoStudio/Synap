use super::trust::public_key_fingerprint;

/// 根据公钥指纹生成一个稳定但更易辨认的颜文字标识。
///
/// 这个标识不是安全校验手段，只适合用在 UI 或日志里帮助用户快速识别
/// 真正的校验仍然应该看完整指纹。
pub fn generate_kaomoji_fingerprint(fingerprint: &[u8]) -> String {
    if fingerprint.len() < 4 {
        return "(?_?)".to_string();
    }

    let contours = [
        ("", ""),
        ("(", ")"),
        ("ヽ(", ")ﾉ"),
        ("o(", ")o"),
        ("q(", ")p"),
        ("ʕ", "ʔ"),
        ("꒰", "꒱"),
        ("【", "】"),
        ("⊂(", ")⊃"),
        ("ᕙ(", ")ᕗ"),
        ("૮", "ა"),
        ("[(", ")]"),
    ];

    let eyes = [
        ("•", "•"),
        ("◕", "◕"),
        ("ಠ", "ಠ"),
        (">", "<"),
        ("T", "T"),
        ("^", "^"),
        ("≖", "≖"),
        ("☆", "☆"),
        ("◉", "◉"),
        ("눈", "눈"),
        ("@", "@"),
        ("♥", "♥"),
        ("x", "x"),
        ("-", "-"),
        ("O", "O"),
        ("+", "+"),
        ("⌐■", "■"),
        ("$ ", " $"),
    ];

    let mouths = [
        "_", ".", "ω", "▽", "△", "﹏", "Д", "x", "o", "v", "~", "‿", " ⌂ ", "∀", "ㅂ", "ᴥ", "ε",
        "罒", "ー",
    ];

    let accessories = [
        "", "✨", "💦", "💢", "🌸", "🔥", "💤", "⚡", "💻", "☕", "🔧", "BUG", "🚀", "📡", "🤖",
        "⭐",
    ];

    let contour = contours[fingerprint[0] as usize % contours.len()];
    let eye = eyes[fingerprint[1] as usize % eyes.len()];
    let mouth = mouths[fingerprint[2] as usize % mouths.len()];
    let accessory = accessories[fingerprint[3] as usize % accessories.len()];

    format!(
        "{}{}{}{}{}{}",
        contour.0, eye.0, mouth, eye.1, contour.1, accessory
    )
}

/// 根据给定公钥生成一个稳定的艺术头像 PNG。
///
/// 输出格式使用 `PNG`，原因是它适合直接通过 `uniffi` 以 `Vec<u8>` 传给
/// Android，再用 `BitmapFactory.decodeByteArray` 解码展示。
pub fn generate_public_key_avatar_png(public_key: &[u8; 32]) -> Vec<u8> {
    const SIZE: usize = 256;
    const GRID_SIZE: usize = 7;

    let fingerprint = public_key_fingerprint(public_key);
    let seed = u32::from_be_bytes([
        fingerprint[0],
        fingerprint[1],
        fingerprint[2],
        fingerprint[3],
    ]);
    let mut rng = Mulberry32::new(seed);
    let palette = generate_palette(&mut rng);
    let params = generate_constellation_params(&mut rng);

    let mut image = RasterImage::new(SIZE, SIZE, palette.surface);
    add_background_glow(&mut image, &palette, &mut rng);

    let margin = SIZE as f32 * 0.18;
    let spacing = (SIZE as f32 - 2.0 * margin) / (GRID_SIZE as f32 - 1.0);

    let mut grid_points = Vec::with_capacity(GRID_SIZE * GRID_SIZE);
    for gy in 0..GRID_SIZE {
        for gx in 0..GRID_SIZE {
            grid_points.push(Point {
                x: margin + gx as f32 * spacing,
                y: margin + gy as f32 * spacing,
            });
        }
    }

    let mut node_rng = Mulberry32::new(seed.wrapping_add(111));
    shuffle(&mut grid_points, &mut node_rng);
    let selected_points = &grid_points[..params.node_count];

    let mut nodes = Vec::with_capacity(selected_points.len());
    for (index, point) in selected_points.iter().enumerate() {
        let radius = if index == 0 {
            8.0 + node_rng.next_f32() * 4.0
        } else if index < 3 {
            5.0 + node_rng.next_f32() * 3.0
        } else {
            3.0 + node_rng.next_f32() * 2.0
        };
        nodes.push(Node {
            center: *point,
            radius,
            star_class: if index == 0 {
                StarClass::Primary
            } else if index < 3 {
                StarClass::Medium
            } else {
                StarClass::Small
            },
            square: node_rng.next_f32() < 0.15,
        });
    }

    let mut edges = Vec::new();
    if !nodes.is_empty() {
        let mut in_tree = vec![false; nodes.len()];
        in_tree[0] = true;
        for target in 1..nodes.len() {
            let mut best_from = 0usize;
            let mut best_dist = f32::MAX;
            for source in 0..target {
                if !in_tree[source] {
                    continue;
                }
                let dist = distance(nodes[source].center, nodes[target].center);
                if dist < best_dist {
                    best_dist = dist;
                    best_from = source;
                }
            }
            edges.push(Edge {
                from: best_from,
                to: target,
            });
            in_tree[target] = true;
        }
    }

    let mut conn_rng = Mulberry32::new(seed.wrapping_add(222));
    for i in 0..nodes.len() {
        for j in (i + 1)..nodes.len() {
            let already_linked = edges
                .iter()
                .any(|edge| (edge.from == i && edge.to == j) || (edge.from == j && edge.to == i));
            if !already_linked && conn_rng.next_f32() < params.connection_density * 0.8 {
                edges.push(Edge { from: i, to: j });
            }
        }
    }

    for edge in &edges {
        let from = nodes[edge.from].center;
        let to = nodes[edge.to].center;
        let color = if params.use_secondary_nodes && conn_rng.next_f32() < 0.3 {
            palette.secondary
        } else {
            palette.primary
        };
        let width = 1.8 + conn_rng.next_f32() * 1.8;
        let dash = params.dash_style && conn_rng.next_f32() < 0.4;
        image.draw_line(from, to, width, color, dash);
    }

    for node in &nodes {
        let color = if matches!(node.star_class, StarClass::Primary) || node_rng.next_f32() > 0.25 {
            palette.primary
        } else {
            palette.secondary
        };

        match node.star_class {
            StarClass::Primary => image.draw_glow(node.center, node.radius * 3.6, color, 0.18),
            StarClass::Medium => image.draw_glow(node.center, node.radius * 2.8, color, 0.1),
            StarClass::Small => {}
        }

        if node.square {
            image.draw_square(node.center, node.radius * 0.9, color);
        } else {
            image.draw_circle(node.center, node.radius, color);
        }
    }

    encode_png_rgba8(SIZE as u32, SIZE as u32, &image.pixels)
}

#[derive(Clone, Copy)]
struct Point {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy)]
struct Rgba {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

#[derive(Clone, Copy)]
struct Palette {
    primary: Rgba,
    secondary: Rgba,
    surface: Rgba,
}

struct ConstellationParams {
    node_count: usize,
    connection_density: f32,
    use_secondary_nodes: bool,
    dash_style: bool,
}

#[derive(Clone, Copy)]
enum StarClass {
    Primary,
    Medium,
    Small,
}

struct Node {
    center: Point,
    radius: f32,
    star_class: StarClass,
    square: bool,
}

struct Edge {
    from: usize,
    to: usize,
}

struct RasterImage {
    width: usize,
    height: usize,
    pixels: Vec<u8>,
}

impl RasterImage {
    fn new(width: usize, height: usize, color: Rgba) -> Self {
        let mut pixels = vec![0_u8; width * height * 4];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk[0] = color.r;
            chunk[1] = color.g;
            chunk[2] = color.b;
            chunk[3] = color.a;
        }
        Self {
            width,
            height,
            pixels,
        }
    }

    fn blend_pixel(&mut self, x: usize, y: usize, src: Rgba, opacity: f32) {
        if x >= self.width || y >= self.height || opacity <= 0.0 {
            return;
        }
        let alpha = (src.a as f32 / 255.0) * opacity.clamp(0.0, 1.0);
        if alpha <= 0.0 {
            return;
        }

        let idx = (y * self.width + x) * 4;
        let dst_r = self.pixels[idx] as f32;
        let dst_g = self.pixels[idx + 1] as f32;
        let dst_b = self.pixels[idx + 2] as f32;

        self.pixels[idx] = blend_channel(dst_r, src.r as f32, alpha);
        self.pixels[idx + 1] = blend_channel(dst_g, src.g as f32, alpha);
        self.pixels[idx + 2] = blend_channel(dst_b, src.b as f32, alpha);
        self.pixels[idx + 3] = 255;
    }

    fn draw_circle(&mut self, center: Point, radius: f32, color: Rgba) {
        self.draw_circle_soft(center, radius, color, 1.0);
    }

    fn draw_circle_soft(&mut self, center: Point, radius: f32, color: Rgba, opacity: f32) {
        let min_x = (center.x - radius - 1.0).floor().max(0.0) as usize;
        let max_x = (center.x + radius + 1.0).ceil().min((self.width - 1) as f32) as usize;
        let min_y = (center.y - radius - 1.0).floor().max(0.0) as usize;
        let max_y = (center.y + radius + 1.0).ceil().min((self.height - 1) as f32) as usize;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let dx = x as f32 + 0.5 - center.x;
                let dy = y as f32 + 0.5 - center.y;
                let dist = (dx * dx + dy * dy).sqrt();
                let coverage = (radius + 0.75 - dist).clamp(0.0, 1.0);
                self.blend_pixel(x, y, color, coverage * opacity);
            }
        }
    }

    fn draw_square(&mut self, center: Point, half_extent: f32, color: Rgba) {
        let min_x = (center.x - half_extent).floor().max(0.0) as usize;
        let max_x = (center.x + half_extent).ceil().min((self.width - 1) as f32) as usize;
        let min_y = (center.y - half_extent).floor().max(0.0) as usize;
        let max_y = (center.y + half_extent).ceil().min((self.height - 1) as f32) as usize;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                self.blend_pixel(x, y, color, 1.0);
            }
        }
    }

    fn draw_glow(&mut self, center: Point, radius: f32, color: Rgba, strength: f32) {
        let mut glow = color;
        glow.a = 255;
        self.draw_circle_soft(center, radius, glow, strength);
    }

    fn draw_line(&mut self, from: Point, to: Point, width: f32, color: Rgba, dashed: bool) {
        let half_width = width / 2.0;
        let min_x = (from.x.min(to.x) - width - 1.0).floor().max(0.0) as usize;
        let max_x = (from.x.max(to.x) + width + 1.0)
            .ceil()
            .min((self.width - 1) as f32) as usize;
        let min_y = (from.y.min(to.y) - width - 1.0).floor().max(0.0) as usize;
        let max_y = (from.y.max(to.y) + width + 1.0)
            .ceil()
            .min((self.height - 1) as f32) as usize;

        let dx = to.x - from.x;
        let dy = to.y - from.y;
        let len_sq = dx * dx + dy * dy;
        if len_sq <= f32::EPSILON {
            self.draw_circle(from, half_width, color);
            return;
        }
        let length = len_sq.sqrt();
        let dash_cycle = 11.0;
        let dash_on = 6.0;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                let px = x as f32 + 0.5;
                let py = y as f32 + 0.5;
                let t = (((px - from.x) * dx + (py - from.y) * dy) / len_sq).clamp(0.0, 1.0);
                let nearest_x = from.x + dx * t;
                let nearest_y = from.y + dy * t;
                let dist = ((px - nearest_x).powi(2) + (py - nearest_y).powi(2)).sqrt();
                if dist > half_width + 0.75 {
                    continue;
                }
                if dashed {
                    let along = t * length;
                    if along % dash_cycle > dash_on {
                        continue;
                    }
                }
                let coverage = (half_width + 0.75 - dist).clamp(0.0, 1.0);
                self.blend_pixel(x, y, color, coverage);
            }
        }
    }
}

struct Mulberry32 {
    state: u32,
}

impl Mulberry32 {
    fn new(seed: u32) -> Self {
        Self { state: seed }
    }

    fn next_u32(&mut self) -> u32 {
        self.state = self.state.wrapping_add(0x6D2B79F5);
        let mut t = self.state;
        t = (t ^ (t >> 15)).wrapping_mul(t | 1);
        t ^= t.wrapping_add((t ^ (t >> 7)).wrapping_mul(t | 61));
        t ^ (t >> 14)
    }

    fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / 4294967296.0
    }
}

fn generate_palette(rng: &mut Mulberry32) -> Palette {
    let primary_hue = (rng.next_f32() * 360.0).floor();
    let secondary_hue = (primary_hue + 30.0 + (rng.next_f32() * 60.0).floor()) % 360.0;
    let saturation = 55.0 + (rng.next_f32() * 35.0).floor();
    let lightness = 45.0 + (rng.next_f32() * 20.0).floor();

    Palette {
        primary: hsl_to_rgba(primary_hue, saturation, lightness, 255),
        secondary: hsl_to_rgba(secondary_hue, saturation - 5.0, lightness + 5.0, 255),
        surface: Rgba {
            r: 0xF7,
            g: 0xF2,
            b: 0xFA,
            a: 255,
        },
    }
}

fn generate_constellation_params(rng: &mut Mulberry32) -> ConstellationParams {
    ConstellationParams {
        node_count: 5 + (rng.next_u32() as usize % 4),
        connection_density: 0.25 + rng.next_f32() * 0.35,
        use_secondary_nodes: rng.next_f32() < 0.4,
        dash_style: rng.next_f32() < 0.25,
    }
}

fn add_background_glow(image: &mut RasterImage, palette: &Palette, rng: &mut Mulberry32) {
    let center = Point {
        x: image.width as f32 * (0.3 + rng.next_f32() * 0.4),
        y: image.height as f32 * (0.3 + rng.next_f32() * 0.4),
    };
    let radius = image.width.min(image.height) as f32 * (0.28 + rng.next_f32() * 0.1);
    image.draw_glow(center, radius, palette.secondary, 0.09);
}

fn shuffle<T>(items: &mut [T], rng: &mut Mulberry32) {
    if items.len() < 2 {
        return;
    }
    for i in (1..items.len()).rev() {
        let j = (rng.next_u32() as usize) % (i + 1);
        items.swap(i, j);
    }
}

fn distance(a: Point, b: Point) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2)).sqrt()
}

fn hsl_to_rgba(h_deg: f32, s_pct: f32, l_pct: f32, a: u8) -> Rgba {
    let h = (h_deg / 360.0).rem_euclid(1.0);
    let s = (s_pct / 100.0).clamp(0.0, 1.0);
    let l = (l_pct / 100.0).clamp(0.0, 1.0);

    if s <= f32::EPSILON {
        let gray = (l * 255.0).round() as u8;
        return Rgba {
            r: gray,
            g: gray,
            b: gray,
            a,
        };
    }

    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;

    let r = hue_to_rgb(p, q, h + 1.0 / 3.0);
    let g = hue_to_rgb(p, q, h);
    let b = hue_to_rgb(p, q, h - 1.0 / 3.0);

    Rgba {
        r: (r * 255.0).round() as u8,
        g: (g * 255.0).round() as u8,
        b: (b * 255.0).round() as u8,
        a,
    }
}

fn hue_to_rgb(p: f32, q: f32, mut t: f32) -> f32 {
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        return p + (q - p) * 6.0 * t;
    }
    if t < 0.5 {
        return q;
    }
    if t < 2.0 / 3.0 {
        return p + (q - p) * (2.0 / 3.0 - t) * 6.0;
    }
    p
}

fn blend_channel(dst: f32, src: f32, alpha: f32) -> u8 {
    (src * alpha + dst * (1.0 - alpha)).round().clamp(0.0, 255.0) as u8
}

fn encode_png_rgba8(width: u32, height: u32, rgba: &[u8]) -> Vec<u8> {
    debug_assert_eq!(rgba.len(), width as usize * height as usize * 4);

    let stride = width as usize * 4;
    let mut raw = Vec::with_capacity((stride + 1) * height as usize);
    for row in 0..height as usize {
        raw.push(0);
        let start = row * stride;
        raw.extend_from_slice(&rgba[start..start + stride]);
    }

    let mut png = Vec::new();
    png.extend_from_slice(&[137, 80, 78, 71, 13, 10, 26, 10]);

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.push(8);
    ihdr.push(6);
    ihdr.push(0);
    ihdr.push(0);
    ihdr.push(0);
    write_png_chunk(&mut png, *b"IHDR", &ihdr);

    let compressed = zlib_encode_stored(&raw);
    write_png_chunk(&mut png, *b"IDAT", &compressed);
    write_png_chunk(&mut png, *b"IEND", &[]);
    png
}

fn zlib_encode_stored(data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(data.len() + 6 + (data.len() / 65535 + 1) * 5);
    out.extend_from_slice(&[0x78, 0x01]);

    let mut offset = 0usize;
    while offset < data.len() {
        let remaining = data.len() - offset;
        let block_len = remaining.min(65_535);
        let is_final = offset + block_len == data.len();
        out.push(if is_final { 0x01 } else { 0x00 });

        let len = block_len as u16;
        let nlen = !len;
        out.extend_from_slice(&len.to_le_bytes());
        out.extend_from_slice(&nlen.to_le_bytes());
        out.extend_from_slice(&data[offset..offset + block_len]);
        offset += block_len;
    }

    let checksum = adler32(data);
    out.extend_from_slice(&checksum.to_be_bytes());
    out
}

fn write_png_chunk(output: &mut Vec<u8>, chunk_type: [u8; 4], data: &[u8]) {
    output.extend_from_slice(&(data.len() as u32).to_be_bytes());
    output.extend_from_slice(&chunk_type);
    output.extend_from_slice(data);

    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&chunk_type);
    crc_input.extend_from_slice(data);
    output.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a = 1u32;
    let mut b = 0u32;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &byte in data {
        crc ^= byte as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg() & 0xEDB8_8320;
            crc = (crc >> 1) ^ mask;
        }
    }
    !crc
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::{generate_kaomoji_fingerprint, generate_public_key_avatar_png};

    #[test]
    fn test_generate_kaomoji_fingerprint_is_stable() {
        let fingerprint = [12u8, 45, 120, 8];
        assert_eq!(generate_kaomoji_fingerprint(&fingerprint), "눈Д눈💻");
    }

    #[test]
    fn test_generate_kaomoji_fingerprint_falls_back_for_short_input() {
        assert_eq!(generate_kaomoji_fingerprint(&[1, 2, 3]), "(?_?)");
    }

    #[test]
    fn test_generate_public_key_avatar_png_is_stable() {
        let public_key = [7u8; 32];
        let png = generate_public_key_avatar_png(&public_key);

        assert!(png.starts_with(&[137, 80, 78, 71, 13, 10, 26, 10]));
        assert_eq!(Sha256::digest(&png[..]).as_slice(), &hex_to_bytes32("6e3eed4ab3d7bd686e976c5f59a5d95862fcb70846c38c278e9a2bd7bf9f0f10"));
    }

    #[test]
    fn test_generate_public_key_avatar_png_differs_by_public_key() {
        let left = generate_public_key_avatar_png(&[1u8; 32]);
        let right = generate_public_key_avatar_png(&[2u8; 32]);
        assert_ne!(left, right);
    }

    fn hex_to_bytes32(hex: &str) -> [u8; 32] {
        let mut out = [0u8; 32];
        for (idx, chunk) in hex.as_bytes().chunks_exact(2).enumerate() {
            out[idx] = (hex_nibble(chunk[0]) << 4) | hex_nibble(chunk[1]);
        }
        out
    }

    fn hex_nibble(byte: u8) -> u8 {
        match byte {
            b'0'..=b'9' => byte - b'0',
            b'a'..=b'f' => byte - b'a' + 10,
            b'A'..=b'F' => byte - b'A' + 10,
            _ => panic!("invalid hex"),
        }
    }
}
