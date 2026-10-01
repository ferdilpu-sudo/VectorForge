use std::cell::Cell;
use std::fmt::Write as _;

use image::{Rgba, RgbaImage};
use svg2pdf::{ConversionOptions, PageOptions};
use vtracer::ir::PathCmd;
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

fn contains_alpha_encoding(svg: &str) -> bool {
    svg.contains("fill-opacity=")
        || svg.contains("opacity=")
        || svg.contains("fill:rgba(")
        || svg.contains("fill="rgba(")
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
        let color = shape.paint.color().to_hex_string();
        let (red, green, blue) = color_to_rgb(&color)?;
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

        // SVG and PostScript both use the nonzero winding rule by default.
        output.push_str("fill\n");
    }

    output.push_str("grestore\nshowpage\n%%EOF\n");
    Ok(output)
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

    let tree = svg2pdf::usvg::Tree::from_str(&svg, &svg2pdf::usvg::Options::default())
        .map_err(|error| format!("SVG parse failed: {error}"))?;
    let pdf = svg2pdf::to_pdf(
        &tree,
        ConversionOptions::default(),
        PageOptions { dpi: 96.0 },
    )
    .map_err(|error| format!("SVG -> PDF failed: {error}"))?;
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

    match pipeline.to_svg(&fully_transparent(48, 48)) {
        Ok(transparent_svg) if !transparent_svg.contains("<path") => {
            println!("[PASS] fully transparent source produces no visible path");
        }
        Ok(_) => {
            println!(
                "[BLOCKED] fully transparent source produced a visible path; inspect before D10"
            );
        }
        Err(error) => {
            println!(
                "[BLOCKED] fully transparent source returned tracer error: {error}"
            );
        }
    }

    match pipeline.to_svg(&partial_alpha(64, 64)) {
        Ok(alpha_svg) if contains_alpha_encoding(&alpha_svg) => {
            println!("[PASS] partial alpha is represented in stock SVG output");
        }
        Ok(_) => {
            println!(
                "[BLOCKED] partial alpha is not represented by stock VTracer SVG output"
            );
        }
        Err(error) => {
            println!("[BLOCKED] partial-alpha trace returned tracer error: {error}");
        }
    }

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
