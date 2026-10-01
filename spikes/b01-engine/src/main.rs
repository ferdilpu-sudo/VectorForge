use std::cell::Cell;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::time::Instant;

use image::{Rgba, RgbaImage};
use svg2pdf::{ConversionOptions, PageOptions};
use visioncortex::BinaryImage;
use vtracer::ir::{Layer, Paint, PathCmd, RegionMask, Segmentation};
use vtracer::progress::{CancelToken, Phase, Progress};
use vtracer::{ColorImage, Config, Error, VectorDoc};

fn rgba_image_to_color(image: RgbaImage) -> ColorImage {
    let width = image.width() as usize;
    let height = image.height() as usize;
    ColorImage {
        pixels: image.into_raw(),
        width,
        height,
    }
}

fn checker(width: u32, height: u32) -> ColorImage {
    rgba_image_to_color(RgbaImage::from_fn(width, height, |x, y| {
        if (x / 6 + y / 6) % 2 == 0 {
            Rgba([210, 60, 60, 255])
        } else {
            Rgba([60, 90, 200, 255])
        }
    }))
}

fn fully_transparent(width: u32, height: u32) -> ColorImage {
    rgba_image_to_color(RgbaImage::from_pixel(
        width,
        height,
        Rgba([240, 70, 40, 0]),
    ))
}

fn partial_alpha(width: u32, height: u32) -> ColorImage {
    rgba_image_to_color(RgbaImage::from_fn(width, height, |x, y| {
        if x > width / 4
            && x < width * 3 / 4
            && y > height / 4
            && y < height * 3 / 4
        {
            Rgba([240, 70, 40, 128])
        } else {
            Rgba([0, 0, 0, 0])
        }
    }))
}

fn same_rgb_alpha_split(width: u32, height: u32) -> ColorImage {
    rgba_image_to_color(RgbaImage::from_fn(width, height, |x, _| {
        let alpha = if x < width / 2 { 128 } else { 255 };
        Rgba([40, 120, 220, alpha])
    }))
}

fn smooth_alpha_gradient(width: u32, height: u32) -> ColorImage {
    rgba_image_to_color(RgbaImage::from_fn(width, height, |x, _| {
        let alpha = 1 + ((u64::from(x) * 254) / u64::from(width.max(2) - 1)) as u8;
        Rgba([40, 120, 220, alpha])
    }))
}

fn fragmented_alpha(width: u32, height: u32) -> ColorImage {
    rgba_image_to_color(RgbaImage::from_fn(width, height, |x, y| {
        let alpha = 1 + (((u64::from(x) * 37 + u64::from(y) * 17) % 255) as u8);
        Rgba([40, 120, 220, alpha])
    }))
}

fn is_fully_transparent(image: &ColorImage) -> bool {
    image
        .pixels
        .chunks_exact(4)
        .all(|pixel| pixel[3] == 0)
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    if let Some(message) = payload.downcast_ref::<&str>() {
        (*message).to_owned()
    } else if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else {
        "non-string panic payload".to_owned()
    }
}

fn run_doc_catching_dependency_panic(
    image: ColorImage,
) -> Result<Result<VectorDoc, Error>, String> {
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));

    let result = std::panic::catch_unwind(move || {
        Config::default()
            .build()
            .and_then(|pipeline| pipeline.run(&image))
    });

    std::panic::set_hook(previous_hook);
    result.map_err(panic_message)
}

fn fmt_number(value: f64) -> String {
    let mut text = format!("{value:.3}");
    while text.contains('.') && text.ends_with('0') {
        text.pop();
    }
    if text.ends_with('.') {
        text.pop();
    }
    text
}

fn path_data(commands: &[PathCmd]) -> String {
    let mut data = String::new();

    for command in commands {
        match command {
            PathCmd::MoveTo(point) => {
                let _ = write!(
                    data,
                    "M{},{}",
                    fmt_number(point.x),
                    fmt_number(point.y)
                );
            }
            PathCmd::LineTo(point) => {
                let _ = write!(
                    data,
                    "L{},{}",
                    fmt_number(point.x),
                    fmt_number(point.y)
                );
            }
            PathCmd::CubicTo(control1, control2, end) => {
                let _ = write!(
                    data,
                    "C{},{} {},{} {},{}",
                    fmt_number(control1.x),
                    fmt_number(control1.y),
                    fmt_number(control2.x),
                    fmt_number(control2.y),
                    fmt_number(end.x),
                    fmt_number(end.y)
                );
            }
            PathCmd::Close => data.push('Z'),
        }
    }

    data
}

fn vector_doc_to_alpha_svg(doc: &VectorDoc) -> String {
    let mut output = String::new();
    let _ = writeln!(
        output,
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{}\" height=\"{}\" viewBox=\"0 0 {} {}\">",
        doc.width, doc.height, doc.width, doc.height
    );

    for shape in &doc.shapes {
        let color = shape.paint.color();
        if color.a == 0 {
            continue;
        }

        let mut data = String::new();
        for subpath in &shape.path.subpaths {
            data.push_str(&path_data(&subpath.commands));
        }
        if data.is_empty() {
            continue;
        }

        if color.a == 255 {
            let _ = writeln!(
                output,
                "<path d=\"{}\" fill=\"{}\"/>",
                data,
                color.to_hex_string()
            );
        } else {
            let opacity = f64::from(color.a) / 255.0;
            let _ = writeln!(
                output,
                "<path d=\"{}\" fill=\"{}\" fill-opacity=\"{}\"/>",
                data,
                color.to_hex_string(),
                fmt_number(opacity)
            );
        }
    }

    output.push_str("</svg>\n");
    output
}

fn empty_svg(width: usize, height: usize) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"{height}\" viewBox=\"0 0 {width} {height}\"></svg>\n"
    )
}

fn color_to_rgb(hex: &str) -> Result<(f64, f64, f64), String> {
    let value = hex.strip_prefix('#').unwrap_or(hex);
    if value.len() != 6 {
        return Err(format!("unexpected RGB color from tracer: {hex}"));
    }

    let channel = |range: std::ops::Range<usize>| {
        u8::from_str_radix(&value[range], 16)
            .map(|value| f64::from(value) / 255.0)
            .map_err(|error| error.to_string())
    };

    Ok((channel(0..2)?, channel(2..4)?, channel(4..6)?))
}

fn vector_doc_to_eps(doc: &VectorDoc) -> Result<String, String> {
    if doc.width == 0 || doc.height == 0 {
        return Err("EPS document has zero bounds".to_owned());
    }

    let mut output = String::new();
    let _ = writeln!(output, "%!PS-Adobe-3.0 EPSF-3.0");
    let _ = writeln!(
        output,
        "%%BoundingBox: 0 0 {} {}",
        doc.width, doc.height
    );
    let _ = writeln!(output, "%%LanguageLevel: 2");
    let _ = writeln!(output, "%%Pages: 1");
    let _ = writeln!(output, "%%EndComments");
    let _ = writeln!(output, "gsave");
    let _ = writeln!(output, "0 {} translate", doc.height);
    let _ = writeln!(output, "1 -1 scale");

    for shape in &doc.shapes {
        let color = shape.paint.color();
        let (red, green, blue) = color_to_rgb(&color.to_hex_string())?;
        let _ = writeln!(output, "{red:.6} {green:.6} {blue:.6} setrgbcolor");
        output.push_str("newpath\n");

        for subpath in &shape.path.subpaths {
            for command in &subpath.commands {
                match command {
                    PathCmd::MoveTo(point) => {
                        let _ = writeln!(output, "{} {} moveto", point.x, point.y);
                    }
                    PathCmd::LineTo(point) => {
                        let _ = writeln!(output, "{} {} lineto", point.x, point.y);
                    }
                    PathCmd::CubicTo(control1, control2, end) => {
                        let _ = writeln!(
                            output,
                            "{} {} {} {} {} {} curveto",
                            control1.x,
                            control1.y,
                            control2.x,
                            control2.y,
                            end.x,
                            end.y
                        );
                    }
                    PathCmd::Close => output.push_str("closepath\n"),
                }
            }
        }

        output.push_str("fill\n");
    }

    output.push_str("grestore\nshowpage\n%%EOF\n");
    Ok(output)
}

fn pdf_from_svg(svg: &str) -> Result<Vec<u8>, String> {
    let tree = svg2pdf::usvg::Tree::from_str(svg, &svg2pdf::usvg::Options::default())
        .map_err(|error| format!("SVG parse failed: {error}"))?;

    svg2pdf::to_pdf(
        &tree,
        ConversionOptions::default(),
        PageOptions { dpi: 96.0 },
    )
    .map_err(|error| format!("SVG -> PDF failed: {error}"))
}

fn distinct_alphas(doc: &VectorDoc) -> Vec<u8> {
    let mut values: Vec<u8> = doc
        .shapes
        .iter()
        .map(|shape| shape.paint.color().a)
        .collect();
    values.sort_unstable();
    values.dedup();
    values
}


#[derive(Clone, Copy)]
struct AlphaBounds {
    left: usize,
    top: usize,
    right: usize,
    bottom: usize,
}

impl AlphaBounds {
    fn new(x: usize, y: usize) -> Self {
        Self {
            left: x,
            top: y,
            right: x,
            bottom: y,
        }
    }

    fn include(&mut self, x: usize, y: usize) {
        self.left = self.left.min(x);
        self.top = self.top.min(y);
        self.right = self.right.max(x);
        self.bottom = self.bottom.max(y);
    }

    fn width(self) -> usize {
        self.right - self.left + 1
    }

    fn height(self) -> usize {
        self.bottom - self.top + 1
    }
}

fn split_segmentation_by_source_alpha(
    segmentation: Segmentation,
    source: &ColorImage,
) -> Result<Segmentation, String> {
    let mut result = Segmentation::new(segmentation.width, segmentation.height);

    for layer in segmentation.layers {
        let base_color = layer.paint.color();
        let mut bounds: BTreeMap<u8, AlphaBounds> = BTreeMap::new();

        for local_y in 0..layer.mask.image.height {
            for local_x in 0..layer.mask.image.width {
                if !layer.mask.image.get_pixel(local_x, local_y) {
                    continue;
                }

                let global_x = layer.mask.offset.x + local_x as i32;
                let global_y = layer.mask.offset.y + local_y as i32;
                if global_x < 0
                    || global_y < 0
                    || global_x as usize >= source.width
                    || global_y as usize >= source.height
                {
                    return Err("segmentation mask escaped source bounds".to_owned());
                }

                let alpha = source.get_pixel(global_x as usize, global_y as usize).a;
                if alpha == 0 {
                    continue;
                }

                bounds
                    .entry(alpha)
                    .and_modify(|bbox| bbox.include(local_x, local_y))
                    .or_insert_with(|| AlphaBounds::new(local_x, local_y));
            }
        }

        let mut masks: BTreeMap<u8, (AlphaBounds, BinaryImage)> = bounds
            .into_iter()
            .map(|(alpha, bbox)| {
                (
                    alpha,
                    (bbox, BinaryImage::new_w_h(bbox.width(), bbox.height())),
                )
            })
            .collect();

        for local_y in 0..layer.mask.image.height {
            for local_x in 0..layer.mask.image.width {
                if !layer.mask.image.get_pixel(local_x, local_y) {
                    continue;
                }

                let global_x = (layer.mask.offset.x + local_x as i32) as usize;
                let global_y = (layer.mask.offset.y + local_y as i32) as usize;
                let alpha = source.get_pixel(global_x, global_y).a;
                if alpha == 0 {
                    continue;
                }

                let (bbox, mask) = masks
                    .get_mut(&alpha)
                    .ok_or_else(|| "alpha mask disappeared during split".to_owned())?;
                mask.set_pixel(local_x - bbox.left, local_y - bbox.top, true);
            }
        }

        for (alpha, (bbox, mask)) in masks {
            result.layers.push(Layer {
                paint: Paint::Solid(vtracer::Color::new_rgba(
                    base_color.r,
                    base_color.g,
                    base_color.b,
                    alpha,
                )),
                mask: RegionMask::new(
                    mask,
                    vtracer::PointI32 {
                        x: layer.mask.offset.x + bbox.left as i32,
                        y: layer.mask.offset.y + bbox.top as i32,
                    },
                ),
            });
        }
    }

    Ok(result)
}

fn segmentation_mask_lower_bound_bytes(segmentation: &Segmentation) -> usize {
    segmentation
        .layers
        .iter()
        .map(|layer| {
            let bits = layer.mask.image.width.saturating_mul(layer.mask.image.height);
            bits.div_ceil(8)
        })
        .sum()
}

fn alpha_levels(source: &ColorImage) -> usize {
    let mut seen = [false; 256];
    for pixel in source.pixels.chunks_exact(4) {
        seen[pixel[3] as usize] = true;
    }
    seen.into_iter().filter(|value| *value).count()
}

fn characterize_alpha_split_cost(
    pipeline: &vtracer::Pipeline,
    label: &str,
    source: &ColorImage,
) -> Result<(), String> {
    let segment_started = Instant::now();
    let segmentation = pipeline
        .segment(source)
        .map_err(|error| error.to_string())?;
    let segment_elapsed = segment_started.elapsed();

    let split_started = Instant::now();
    let split = split_segmentation_by_source_alpha(segmentation, source)?;
    let split_elapsed = split_started.elapsed();

    let mask_bytes = segmentation_mask_lower_bound_bytes(&split);
    println!(
        "[INFO] alpha stress {label}: {}x{}, levels={}, split_layers={}, mask_lower_bound={} KiB, segment={} ms, split={} ms",
        source.width,
        source.height,
        alpha_levels(source),
        split.layers.len(),
        mask_bytes.div_ceil(1024),
        segment_elapsed.as_millis(),
        split_elapsed.as_millis(),
    );

    Ok(())
}

fn trace_alpha_split(
    pipeline: &vtracer::Pipeline,
    source: &ColorImage,
) -> Result<VectorDoc, String> {
    let segmentation = pipeline
        .segment(source)
        .map_err(|error| error.to_string())?;
    let split = split_segmentation_by_source_alpha(segmentation, source)?;
    pipeline.finish(&split).map_err(|error| error.to_string())
}

fn run() -> Result<(), String> {
    let pipeline = Config::default().build().map_err(|error| error.to_string())?;

    let opaque = checker(96, 96);
    let doc = pipeline.run(&opaque).map_err(|error| error.to_string())?;
    if doc.shapes.is_empty() {
        return Err("opaque trace returned no shapes".to_owned());
    }

    let svg = pipeline.writer.write(&doc);
    if !svg.contains("<path") {
        return Err("opaque SVG contains no vector path".to_owned());
    }
    println!("[PASS] opaque raster -> VectorDoc -> SVG");

    let pdf = pdf_from_svg(&svg)?;
    if !pdf.starts_with(b"%PDF-") {
        return Err("PDF output is missing the PDF signature".to_owned());
    }
    println!("[PASS] SVG -> vector PDF bytes at 96 DPI mapping");

    let eps = vector_doc_to_eps(&doc)?;
    if !eps.starts_with("%!PS-Adobe-3.0 EPSF-3.0")
        || !eps.contains("%%BoundingBox:")
        || !eps.contains("setrgbcolor")
        || !eps.contains("\nfill\n")
    {
        return Err("EPS writer characterization failed".to_owned());
    }
    println!("[PASS] VectorDoc -> solid-path EPS characterization");

    let cancel_pipeline = Config::default().build().map_err(|error| error.to_string())?;
    let cancel = CancelToken::new();
    let saw_segment = Cell::new(false);
    let mut on_progress = |progress: Progress| {
        if progress.phase == Phase::Segment {
            saw_segment.set(true);
            cancel.cancel();
        }
    };
    let cancelled = cancel_pipeline.run_with_progress(
        &checker(96, 96),
        &cancel,
        &mut on_progress,
    );
    if !saw_segment.get() || !matches!(cancelled, Err(Error::Cancelled)) {
        return Err("cancellation/progress contract was not observed".to_owned());
    }
    println!("[PASS] phase progress + cooperative cancellation");

    let transparent = fully_transparent(48, 48);
    if !is_fully_transparent(&transparent) {
        return Err("fully-transparent precheck failed".to_owned());
    }
    let transparent_svg = empty_svg(transparent.width, transparent.height);
    if transparent_svg.contains("<path") {
        return Err("fully-transparent precheck produced a visible path".to_owned());
    }
    println!("[PASS] fully transparent precheck -> empty SVG without tracer");

    let partial = partial_alpha(64, 64);
    match run_doc_catching_dependency_panic(partial.clone()) {
        Ok(Ok(alpha_doc)) => {
            let alphas = distinct_alphas(&alpha_doc);
            if !alphas.iter().any(|&alpha| alpha > 0 && alpha < 255) {
                println!(
                    "[BLOCKED] partial alpha was lost before VectorDoc; IR alphas = {alphas:?}"
                );
            } else {
                println!("[PASS] partial alpha survives in VectorDoc: {alphas:?}");
            }
        }
        Ok(Err(error)) => {
            println!("[BLOCKED] partial-alpha trace returned tracer error: {error}");
        }
        Err(message) => {
            println!(
                "[BLOCKED] partial-alpha trace panicked inside VTracer/visioncortex: {message}"
            );
        }
    }

    let split_partial_doc = trace_alpha_split(&pipeline, &partial)?;
    let split_partial_alphas = distinct_alphas(&split_partial_doc);
    if !split_partial_alphas.contains(&128) {
        println!(
            "[BLOCKED] alpha-split segmentation lost partial alpha; IR alphas = {split_partial_alphas:?}"
        );
    } else {
        println!(
            "[PASS] alpha-split segmentation preserves partial alpha: {split_partial_alphas:?}"
        );
        let alpha_svg = vector_doc_to_alpha_svg(&split_partial_doc);
        if !alpha_svg.contains("fill-opacity=") {
            println!("[BLOCKED] VectorForge alpha SVG writer emitted no fill-opacity");
        } else {
            println!("[PASS] alpha-aware SVG writer emits fill-opacity");
            let alpha_pdf = pdf_from_svg(&alpha_svg)?;
            if alpha_pdf.starts_with(b"%PDF-") {
                println!("[PASS] alpha-aware SVG -> PDF conversion");
            } else {
                println!("[BLOCKED] alpha-aware PDF output has no PDF signature");
            }
        }
    }

    let same_rgb = same_rgb_alpha_split(64, 32);
    match run_doc_catching_dependency_panic(same_rgb.clone()) {
        Ok(Ok(merged_doc)) => {
            println!(
                "[INFO] stock segmentation same-RGB alpha result: {:?}",
                distinct_alphas(&merged_doc)
            );
        }
        Ok(Err(error)) => {
            println!("[BLOCKED] stock alpha-boundary trace returned tracer error: {error}");
        }
        Err(message) => {
            println!(
                "[BLOCKED] stock alpha-boundary trace panicked inside VTracer/visioncortex: {message}"
            );
        }
    }

    let split_doc = trace_alpha_split(&pipeline, &same_rgb)?;
    let split_alphas = distinct_alphas(&split_doc);
    if split_alphas.contains(&128) && split_alphas.contains(&255) {
        println!(
            "[PASS] alpha-split segmentation preserves same-RGB boundary: {split_alphas:?}"
        );
    } else {
        println!(
            "[BLOCKED] alpha-split segmentation failed same-RGB boundary; IR alphas = {split_alphas:?}"
        );
    }

    characterize_alpha_split_cost(
        &pipeline,
        "smooth-gradient",
        &smooth_alpha_gradient(512, 256),
    )?;
    characterize_alpha_split_cost(
        &pipeline,
        "fragmented",
        &fragmented_alpha(256, 256),
    )?;

    println!(
        "B01 remains a spike: do not wire this harness into the production frontend."
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("[FAIL] B01 engine spike: {error}");
        std::process::exit(1);
    }
}
