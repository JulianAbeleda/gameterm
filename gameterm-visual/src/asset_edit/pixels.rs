use super::{
    SceneAssetBlendMode, SceneAssetColorChannel, SceneAssetEditError, SceneAssetNormalizedPoint,
    SceneAssetNormalizedRect, SceneAssetPixelRect,
};
use image::imageops::FilterType;
use image::{ImageBuffer, Rgba, RgbaImage};

pub(crate) fn pixel_len(image: &RgbaImage) -> usize {
    image.width() as usize * image.height() as usize
}

pub(crate) fn pixel_index(image: &RgbaImage, x: u32, y: u32) -> usize {
    y as usize * image.width() as usize + x as usize
}

pub(crate) fn parse_rgba(color: &str) -> Result<Rgba<u8>, SceneAssetEditError> {
    let trimmed = color.trim();
    let hex = trimmed.strip_prefix('#').unwrap_or(trimmed);
    if hex.len() != 6 && hex.len() != 8 {
        return Err(SceneAssetEditError::InvalidColor(color.to_string()));
    }
    let r = parse_hex_byte(color, &hex[0..2])?;
    let g = parse_hex_byte(color, &hex[2..4])?;
    let b = parse_hex_byte(color, &hex[4..6])?;
    let a = if hex.len() == 8 {
        parse_hex_byte(color, &hex[6..8])?
    } else {
        255
    };
    Ok(Rgba([r, g, b, a]))
}

fn parse_hex_byte(color: &str, text: &str) -> Result<u8, SceneAssetEditError> {
    u8::from_str_radix(text, 16).map_err(|_| SceneAssetEditError::InvalidColor(color.to_string()))
}

pub(crate) fn normalized_point_to_pixel(
    point: SceneAssetNormalizedPoint,
    width: u32,
    height: u32,
) -> Result<(u32, u32), SceneAssetEditError> {
    if !point.x.is_finite()
        || !point.y.is_finite()
        || point.x < 0.0
        || point.y < 0.0
        || point.x > 1.0
        || point.y > 1.0
    {
        return Err(SceneAssetEditError::InvalidOperation(
            "seed point must be finite and inside 0..1".to_string(),
        ));
    }
    if width == 0 || height == 0 {
        return Err(SceneAssetEditError::InvalidOperation(
            "image dimensions must be non-zero".to_string(),
        ));
    }
    Ok((
        (point.x * width.saturating_sub(1) as f32).round() as u32,
        (point.y * height.saturating_sub(1) as f32).round() as u32,
    ))
}

pub(crate) fn erase_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, soften: u32) {
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let pixel = image.get_pixel_mut(x, y);
            if soften == 0 {
                pixel[3] = 0;
                continue;
            }
            let edge = (x - rect.x)
                .min(rect.right().saturating_sub(x + 1))
                .min(y - rect.y)
                .min(rect.bottom().saturating_sub(y + 1));
            if edge >= soften {
                pixel[3] = 0;
            } else {
                let factor = edge as f32 / soften.max(1) as f32;
                pixel[3] = (pixel[3] as f32 * factor).round() as u8;
            }
        }
    }
}

pub(crate) fn fill_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, color: Rgba<u8>) {
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            blend_pixel(image.get_pixel_mut(x, y), color, 1.0);
        }
    }
}

pub(crate) fn draw_line_in_region(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    from: SceneAssetNormalizedPoint,
    to: SceneAssetNormalizedPoint,
    color: Rgba<u8>,
    width: u32,
) {
    let start = point_in_rect(rect, from);
    let end = point_in_rect(rect, to);
    draw_line(image, start, end, color, width.max(1));
}

pub(crate) fn draw_line(
    image: &mut RgbaImage,
    from: (i32, i32),
    to: (i32, i32),
    color: Rgba<u8>,
    width: u32,
) {
    let dx = (to.0 - from.0).abs();
    let dy = -(to.1 - from.1).abs();
    let sx = if from.0 < to.0 { 1 } else { -1 };
    let sy = if from.1 < to.1 { 1 } else { -1 };
    let mut err = dx + dy;
    let mut x = from.0;
    let mut y = from.1;
    loop {
        draw_disk(image, x, y, width as i32, color);
        if x == to.0 && y == to.1 {
            break;
        }
        let e2 = 2 * err;
        if e2 >= dy {
            err += dy;
            x += sx;
        }
        if e2 <= dx {
            err += dx;
            y += sy;
        }
    }
}

pub(crate) fn draw_normalized_line(
    image: &mut RgbaImage,
    from: SceneAssetNormalizedPoint,
    to: SceneAssetNormalizedPoint,
    color: Rgba<u8>,
    width: u32,
) -> Result<(), SceneAssetEditError> {
    let from = normalized_point_to_pixel(from, image.width(), image.height())?;
    let to = normalized_point_to_pixel(to, image.width(), image.height())?;
    draw_line(
        image,
        (from.0 as i32, from.1 as i32),
        (to.0 as i32, to.1 as i32),
        color,
        width,
    );
    Ok(())
}

pub(crate) fn draw_normalized_path(
    image: &mut RgbaImage,
    path: &[SceneAssetNormalizedPoint],
    color: Rgba<u8>,
    width: u32,
    closed: bool,
) -> Result<(), SceneAssetEditError> {
    if path.len() < 2 {
        return Err(SceneAssetEditError::InvalidOperation(
            "stroke path requires at least two points".to_string(),
        ));
    }
    for pair in path.windows(2) {
        draw_normalized_line(image, pair[0], pair[1], color, width)?;
    }
    if closed && path.len() > 2 {
        draw_normalized_line(image, path[path.len() - 1], path[0], color, width)?;
    }
    Ok(())
}

pub(crate) fn draw_rect_outline(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    color: Rgba<u8>,
    width: u32,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let left = rect.x as i32;
    let right = rect.right().saturating_sub(1) as i32;
    let top = rect.y as i32;
    let bottom = rect.bottom().saturating_sub(1) as i32;
    draw_line(image, (left, top), (right, top), color, width);
    draw_line(image, (right, top), (right, bottom), color, width);
    draw_line(image, (right, bottom), (left, bottom), color, width);
    draw_line(image, (left, bottom), (left, top), color, width);
}

fn draw_disk(image: &mut RgbaImage, cx: i32, cy: i32, radius: i32, color: Rgba<u8>) {
    let radius = radius.max(1);
    let r2 = radius * radius;
    for y in (cy - radius)..=(cy + radius) {
        for x in (cx - radius)..=(cx + radius) {
            if (x - cx) * (x - cx) + (y - cy) * (y - cy) <= r2 {
                if let Some(pixel) = pixel_mut_checked(image, x, y) {
                    blend_pixel(pixel, color, 1.0);
                }
            }
        }
    }
}

pub(crate) fn draw_ellipse(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    stroke: Option<Rgba<u8>>,
    fill: Option<Rgba<u8>>,
    stroke_width: u32,
) {
    let cx = rect.x as f32 + rect.w as f32 / 2.0;
    let cy = rect.y as f32 + rect.h as f32 / 2.0;
    let rx = (rect.w as f32 / 2.0).max(1.0);
    let ry = (rect.h as f32 / 2.0).max(1.0);
    let stroke_band = stroke_width.max(1) as f32 / rx.min(ry).max(1.0);
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let nx = (x as f32 + 0.5 - cx) / rx;
            let ny = (y as f32 + 0.5 - cy) / ry;
            let distance = nx * nx + ny * ny;
            if distance <= 1.0 {
                if let Some(fill_color) = fill {
                    blend_pixel(image.get_pixel_mut(x, y), fill_color, 1.0);
                }
                if let Some(stroke_color) = stroke {
                    if distance >= (1.0 - stroke_band).max(0.0) {
                        blend_pixel(image.get_pixel_mut(x, y), stroke_color, 1.0);
                    }
                }
            }
        }
    }
}

pub(crate) fn composite_scaled(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    overlay: &RgbaImage,
    opacity: f32,
) {
    if rect.w == 0 || rect.h == 0 {
        return;
    }
    let resized = image::imageops::resize(overlay, rect.w, rect.h, FilterType::Lanczos3);
    for y in 0..resized.height() {
        for x in 0..resized.width() {
            let target_x = rect.x + x;
            let target_y = rect.y + y;
            if target_x < image.width() && target_y < image.height() {
                blend_pixel(
                    image.get_pixel_mut(target_x, target_y),
                    *resized.get_pixel(x, y),
                    opacity,
                );
            }
        }
    }
}

pub(crate) fn translate_region(image: &mut RgbaImage, rect: SceneAssetPixelRect, dx: i32, dy: i32) {
    let copy = crop_region(image, rect);
    erase_region(image, rect, 0);
    paste_region(image, &copy, rect.x as i32 + dx, rect.y as i32 + dy, 1.0);
}

pub(crate) fn scale_region(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    sx: f32,
    sy: f32,
) -> Result<(), SceneAssetEditError> {
    if sx <= 0.0 || sy <= 0.0 || !sx.is_finite() || !sy.is_finite() {
        return Err(SceneAssetEditError::InvalidOperation(
            "scale_region sx and sy must be finite positive values".to_string(),
        ));
    }
    let copy = crop_region(image, rect);
    let new_w = ((rect.w as f32 * sx).round() as u32).max(1);
    let new_h = ((rect.h as f32 * sy).round() as u32).max(1);
    let resized = image::imageops::resize(&copy, new_w, new_h, FilterType::Lanczos3);
    erase_region(image, rect, 0);
    let x = rect.x as i32 + (rect.w as i32 - new_w as i32) / 2;
    let y = rect.y as i32 + (rect.h as i32 - new_h as i32) / 2;
    paste_region(image, &resized, x, y, 1.0);
    Ok(())
}

pub(crate) fn multiply_alpha(image: &mut RgbaImage, rect: SceneAssetPixelRect, alpha: f32) {
    let alpha = alpha.clamp(0.0, 1.0);
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let pixel = image.get_pixel_mut(x, y);
            pixel[3] = (pixel[3] as f32 * alpha).round().clamp(0.0, 255.0) as u8;
        }
    }
}

pub(crate) fn tint_region(
    image: &mut RgbaImage,
    rect: SceneAssetPixelRect,
    color: Rgba<u8>,
    amount: f32,
) {
    let amount = amount.clamp(0.0, 1.0);
    for y in rect.y..rect.bottom().min(image.height()) {
        for x in rect.x..rect.right().min(image.width()) {
            let pixel = image.get_pixel_mut(x, y);
            for channel in 0..3 {
                pixel[channel] =
                    lerp(pixel[channel] as f32, color[channel] as f32, amount).round() as u8;
            }
        }
    }
}

pub(crate) fn crop_region(image: &RgbaImage, rect: SceneAssetPixelRect) -> RgbaImage {
    let mut output = ImageBuffer::from_pixel(rect.w, rect.h, Rgba([0, 0, 0, 0]));
    for y in 0..rect.h {
        for x in 0..rect.w {
            let source_x = rect.x + x;
            let source_y = rect.y + y;
            if source_x < image.width() && source_y < image.height() {
                output.put_pixel(x, y, *image.get_pixel(source_x, source_y));
            }
        }
    }
    output
}

pub(crate) fn paste_region(image: &mut RgbaImage, patch: &RgbaImage, x: i32, y: i32, opacity: f32) {
    for patch_y in 0..patch.height() {
        for patch_x in 0..patch.width() {
            let target_x = x + patch_x as i32;
            let target_y = y + patch_y as i32;
            if let Some(pixel) = pixel_mut_checked(image, target_x, target_y) {
                blend_pixel(pixel, *patch.get_pixel(patch_x, patch_y), opacity);
            }
        }
    }
}

pub(crate) fn paste_layer(
    image: &mut RgbaImage,
    patch: &RgbaImage,
    x: i32,
    y: i32,
    opacity: f32,
    blend: SceneAssetBlendMode,
) {
    for patch_y in 0..patch.height() {
        for patch_x in 0..patch.width() {
            let target_x = x + patch_x as i32;
            let target_y = y + patch_y as i32;
            if let Some(pixel) = pixel_mut_checked(image, target_x, target_y) {
                blend_pixel_mode(pixel, *patch.get_pixel(patch_x, patch_y), opacity, blend);
            }
        }
    }
}

pub(crate) fn blend_pixel(dest: &mut Rgba<u8>, src: Rgba<u8>, opacity: f32) {
    let src_alpha = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    if src_alpha <= 0.0 {
        return;
    }
    let dest_alpha = dest[3] as f32 / 255.0;
    let out_alpha = src_alpha + dest_alpha * (1.0 - src_alpha);
    if out_alpha <= 0.0 {
        *dest = Rgba([0, 0, 0, 0]);
        return;
    }
    for channel in 0..3 {
        let src_c = src[channel] as f32 / 255.0;
        let dest_c = dest[channel] as f32 / 255.0;
        let out = (src_c * src_alpha + dest_c * dest_alpha * (1.0 - src_alpha)) / out_alpha;
        dest[channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dest[3] = (out_alpha * 255.0).round().clamp(0.0, 255.0) as u8;
}

fn blend_pixel_mode(dest: &mut Rgba<u8>, src: Rgba<u8>, opacity: f32, blend: SceneAssetBlendMode) {
    if blend == SceneAssetBlendMode::Normal {
        blend_pixel(dest, src, opacity);
        return;
    }
    let alpha = (src[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0);
    if alpha <= 0.0 {
        return;
    }
    for channel in 0..3 {
        let s = src[channel] as f32 / 255.0;
        let d = dest[channel] as f32 / 255.0;
        let blended = match blend {
            SceneAssetBlendMode::Normal => s,
            SceneAssetBlendMode::Add => (d + s).clamp(0.0, 1.0),
            SceneAssetBlendMode::Multiply => d * s,
            SceneAssetBlendMode::Screen => 1.0 - (1.0 - d) * (1.0 - s),
        };
        let out = lerp(d, blended, alpha);
        dest[channel] = (out * 255.0).round().clamp(0.0, 255.0) as u8;
    }
    dest[3] = ((dest[3] as f32 / 255.0) + alpha * (1.0 - dest[3] as f32 / 255.0))
        .mul_add(255.0, 0.0)
        .round()
        .clamp(0.0, 255.0) as u8;
}

fn pixel_mut_checked(image: &mut RgbaImage, x: i32, y: i32) -> Option<&mut Rgba<u8>> {
    if x < 0 || y < 0 {
        return None;
    }
    let x = x as u32;
    let y = y as u32;
    if x < image.width() && y < image.height() {
        Some(image.get_pixel_mut(x, y))
    } else {
        None
    }
}

pub(crate) fn normalized_rect_arg(
    rect: Option<SceneAssetNormalizedRect>,
    label: &str,
) -> Result<SceneAssetNormalizedRect, SceneAssetEditError> {
    rect.ok_or_else(|| SceneAssetEditError::InvalidOperation(format!("{label} requires --rect")))
}

pub(crate) fn changed_pixel_count(a: &RgbaImage, b: &RgbaImage) -> usize {
    if a.dimensions() != b.dimensions() {
        return a.width() as usize * a.height() as usize;
    }
    a.pixels()
        .zip(b.pixels())
        .filter(|(a, b)| a.0 != b.0)
        .count()
}

pub(crate) fn color_channels(channel: SceneAssetColorChannel) -> &'static [usize] {
    match channel {
        SceneAssetColorChannel::Rgb => &[0, 1, 2],
        SceneAssetColorChannel::R => &[0],
        SceneAssetColorChannel::G => &[1],
        SceneAssetColorChannel::B => &[2],
        SceneAssetColorChannel::A => &[3],
    }
}

pub(crate) fn point_in_rect(
    rect: SceneAssetPixelRect,
    point: SceneAssetNormalizedPoint,
) -> (i32, i32) {
    let x = rect.x as f32 + point.x.clamp(0.0, 1.0) * rect.w.saturating_sub(1) as f32;
    let y = rect.y as f32 + point.y.clamp(0.0, 1.0) * rect.h.saturating_sub(1) as f32;
    (x.round() as i32, y.round() as i32)
}

pub(crate) fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}
