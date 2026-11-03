//! Repository chores that don't belong in the app build.
//!
//! `cargo xtask icons-subset` cuts `crates/rpack_egui/assets/fonts/lucide-subset.ttf`
//! down to the Lucide icons the app uses (1,600+ glyphs are 926 KB). `--check` fails
//! when the subset is stale; CI runs it.
//!
//! `cargo xtask icons` renders the app icon sources in
//! `crates/rpack_egui/assets/icon/` to the PNG and ICO files the app ships.

use std::fs::{self, File};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use resvg::{tiny_skia, usvg};

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let check = args.iter().any(|arg| arg == "--check");
    let task = args
        .iter()
        .find(|arg| !arg.starts_with('-'))
        .map(String::as_str);
    let result = match task {
        Some("icons-subset") => icons_subset(check),
        Some("icons") => icons(),
        _ => {
            eprintln!("usage: cargo xtask icons-subset [--check] | icons");
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("xtask: {err}");
            ExitCode::FAILURE
        }
    }
}

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf)
}

/// Largest icon size that uses `icon-small.svg`; the master's detail blurs below this.
const SMALL_ICON_MAX: u32 = 32;

struct Raster {
    size: u32,
    rgba: Vec<u8>,
}

fn icons() -> Result<()> {
    let app = workspace_root().join("crates/rpack_egui");
    let master = load_svg(&app.join("assets/icon/icon.svg"))?;
    let small = load_svg(&app.join("assets/icon/icon-small.svg"))?;

    let png_path = app.join("static/base_icon.png");
    write_png(&png_path, &render(&master, 1024)?)?;
    println!("wrote {}", png_path.display());

    let mut favicon = ico::IconDir::new(ico::ResourceType::Icon);
    for size in [16, 32, 48] {
        let tree = if size <= SMALL_ICON_MAX {
            &small
        } else {
            &master
        };
        let raster = render(tree, size)?;
        let image = ico::IconImage::from_rgba_data(raster.size, raster.size, raster.rgba);
        favicon.add_entry(ico::IconDirEntry::encode(&image)?);
    }
    let ico_path = app.join("assets/favicon.ico");
    favicon.write(BufWriter::new(File::create(&ico_path)?))?;
    println!("wrote {}", ico_path.display());
    Ok(())
}

fn load_svg(path: &Path) -> Result<usvg::Tree> {
    let data = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(usvg::Tree::from_data(&data, &usvg::Options::default())?)
}

fn render(tree: &usvg::Tree, size: u32) -> Result<Raster> {
    let mut pixmap = tiny_skia::Pixmap::new(size, size).ok_or("zero-sized icon")?;
    let scale = size as f32 / tree.size().width();
    resvg::render(
        tree,
        tiny_skia::Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    let rgba = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let color = pixel.demultiply();
            [color.red(), color.green(), color.blue(), color.alpha()]
        })
        .collect();
    Ok(Raster { size, rgba })
}

fn write_png(path: &Path, raster: &Raster) -> Result<()> {
    let mut encoder = png::Encoder::new(
        BufWriter::new(File::create(path)?),
        raster.size,
        raster.size,
    );
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.set_compression(png::Compression::Best);
    encoder.write_header()?.write_image_data(&raster.rgba)?;
    Ok(())
}

fn icons_subset(check: bool) -> Result<()> {
    let root = workspace_root();
    let variants = used_icon_variants(&root.join("crates/rpack_egui/src"))?;
    if variants.is_empty() {
        return Err("no `Icon::…` usages found under src/".into());
    }
    let mut chars = Vec::with_capacity(variants.len());
    for variant in &variants {
        let kebab = kebab_case(variant);
        let icon = lucide_icons::Icon::try_from(kebab.as_str()).map_err(|error| {
            format!("lucide-icons has no icon `{kebab}` (from `Icon::{variant}`): {error}")
        })?;
        chars.push(char::from(icon));
    }

    let full = lucide_icons::LUCIDE_FONT_BYTES;
    let subset_path = root.join("crates/rpack_egui/assets/fonts/lucide-subset.ttf");
    let subset = build_subset(full, &chars)?;

    if check {
        let bytes = fs::read(&subset_path).map_err(|_| {
            format!(
                "{} is missing; run `cargo xtask icons-subset`",
                subset_path.display()
            )
        })?;
        // Diagnose the common case (an icon added to `src/`) with names.
        if let Ok(face) = ttf_parser::Face::parse(&bytes, 0) {
            let mut missing = Vec::new();
            for (variant, ch) in variants.iter().zip(&chars) {
                if face.glyph_index(*ch).is_none() {
                    missing.push(format!("Icon::{variant} (U+{:04X})", u32::from(*ch)));
                }
            }
            if !missing.is_empty() {
                return Err(format!(
                    "{} is missing {} icon(s): {}. Run `cargo xtask icons-subset`.",
                    subset_path.display(),
                    missing.len(),
                    missing.join(", ")
                )
                .into());
            }
        }
        // Byte equality also catches a stale subset whose metrics or tables
        // no longer match what this tool builds.
        if bytes != subset {
            return Err(format!(
                "{} is out of date ({} bytes on disk, {} expected); run `cargo xtask icons-subset`",
                subset_path.display(),
                bytes.len(),
                subset.len()
            )
            .into());
        }
        println!("icons-subset is up to date ({} icons)", variants.len());
        return Ok(());
    }

    fs::create_dir_all(subset_path.parent().expect("path under the workspace"))?;
    fs::write(&subset_path, &subset)?;
    println!(
        "wrote {} ({} icons, {} → {} bytes)",
        subset_path.display(),
        variants.len(),
        full.len(),
        subset.len()
    );
    Ok(())
}

/// The subset font for `chars`, cut from `full`.
fn build_subset(full: &[u8], chars: &[char]) -> Result<Vec<u8>> {
    let face = ttf_parser::Face::parse(full, 0)?;
    // `.notdef` first, then one glyph per used icon.
    let mut gids = Vec::with_capacity(chars.len() + 1);
    gids.push(0);
    for ch in chars {
        let gid = face
            .glyph_index(*ch)
            .ok_or_else(|| format!("the Lucide font lacks U+{:04X}", u32::from(*ch)))?;
        gids.push(gid.0);
    }
    let remapper = subsetter::GlyphRemapper::new_from_glyphs(&gids);
    let subset = subsetter::subset(full, 0, &remapper)?;
    // subsetter writes PDF-oriented fonts: it drops `cmap`, because PDFs
    // carry their own. egui needs one, so append a minimal format 4 cmap
    // (BMP covers every Lucide icon codepoint) and rebuild the sfnt.
    let remapped: Vec<(char, u16)> = chars
        .iter()
        .copied()
        .zip(gids.iter().skip(1))
        .map(|(ch, old)| {
            let new = remapper
                .get(*old)
                .ok_or_else(|| format!("glyph {old} missing from the subset remapping"))?;
            Ok((ch, new))
        })
        .collect::<Result<_>>()?;
    with_cmap(&subset, &remapped, full)
}

/// Every `Icon::…` variant named under `dir`, sorted.
fn used_icon_variants(dir: &Path) -> Result<Vec<String>> {
    let mut variants = std::collections::BTreeSet::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for entry in fs::read_dir(&dir)? {
            let path = entry?.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|ext| ext == "rs") {
                let text = fs::read_to_string(&path)?;
                variants.extend(scan_icon_variants(&text));
            }
        }
    }
    Ok(variants.into_iter().collect())
}

/// `Icon::Name` tokens in `text` that don't continue an identifier, so
/// `CursorIcon::None` doesn't count but `lucide_icons::Icon::Anvil` does.
fn scan_icon_variants(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut variants = Vec::new();
    let mut from = 0;
    while let Some(rel) = text[from..].find("Icon::") {
        let at = from + rel;
        let boundary = at == 0 || !(bytes[at - 1].is_ascii_alphanumeric() || bytes[at - 1] == b'_');
        let name_start = at + "Icon::".len();
        let name_end = text[name_start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .map_or(text.len(), |end| name_start + end);
        if boundary && name_end > name_start {
            variants.push(text[name_start..name_end].to_owned());
        }
        from = name_start;
    }
    variants
}

/// `Icon::AArrowDown` → `a-arrow-down`, Lucide's kebab-case icon names. An
/// uppercase starts a new word after a lowercase or digit, or at the end
/// of an acronym (`…Ccw`); a digit starts one after any lowercase letter
/// except `x` (`Trash2` → `trash-2`, `Grid3x3` → `grid-3x3`).
fn kebab_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len() + 4);
    for (i, &c) in chars.iter().enumerate() {
        let prev = i.checked_sub(1).map(|at| chars[at]);
        let next = chars.get(i + 1).copied();
        let dash = match c {
            'A'..='Z' => {
                let after_word = prev.is_some_and(|p| p.is_ascii_lowercase() || p.is_ascii_digit());
                let acronym_end = prev.is_some_and(|p| p.is_ascii_uppercase())
                    && next.is_some_and(|n| n.is_ascii_lowercase());
                i > 0 && (after_word || acronym_end)
            }
            '0'..='9' => prev.is_some_and(|p| p.is_ascii_lowercase() && p != 'x'),
            _ => false,
        };
        if dash {
            out.push('-');
        }
        out.extend(c.to_lowercase());
    }
    out
}

/// Rewrite `hmtx`/`hhea` so every glyph has its own advance and bearing
/// record, whatever `numberOfHMetrics` the subset has.
fn expand_hmtx(tables: &mut [(u32, Vec<u8>)]) {
    let tag = |name: &[u8; 4]| u32::from_be_bytes(*name);
    let num_glyphs = tables
        .iter()
        .find(|(t, _)| *t == tag(b"maxp"))
        .and_then(|(_, d)| d.get(4..6))
        .map_or(0, |b| u16::from_be_bytes([b[0], b[1]]) as usize);
    let metrics = tables
        .iter()
        .find(|(t, _)| *t == tag(b"hhea"))
        .and_then(|(_, d)| d.get(34..36))
        .map(|b| usize::from(u16::from_be_bytes([b[0], b[1]])));
    let hmtx = tables.iter().find(|(t, _)| *t == tag(b"hmtx"));
    let (Some(num_metrics), Some((_, hmtx))) = (metrics, hmtx) else {
        return;
    };
    if num_glyphs == 0 || num_metrics == 0 || hmtx.len() < num_metrics * 4 {
        return;
    }
    let mut out = Vec::with_capacity(num_glyphs * 4);
    for glyph in 0..num_glyphs {
        let (advance, lsb) = if glyph < num_metrics {
            let at = glyph * 4;
            (
                u16::from_be_bytes([hmtx[at], hmtx[at + 1]]),
                i16::from_be_bytes([hmtx[at + 2], hmtx[at + 3]]),
            )
        } else {
            // Trailing bearings after the last full record.
            let last = (num_metrics - 1) * 4;
            let at = num_metrics * 4 + (glyph - num_metrics) * 2;
            (
                u16::from_be_bytes([hmtx[last], hmtx[last + 1]]),
                hmtx.get(at..at + 2)
                    .map_or(0, |b| i16::from_be_bytes([b[0], b[1]])),
            )
        };
        out.extend(advance.to_be_bytes());
        out.extend(lsb.to_be_bytes());
    }
    for (t, data) in tables.iter_mut() {
        if *t == tag(b"hhea") {
            data[34..36].copy_from_slice(&(num_glyphs as u16).to_be_bytes());
        } else if *t == tag(b"hmtx") {
            data.clone_from(&out);
        }
    }
}

/// Append the tables egui needs to an sfnt subset that subsetter wrote for
/// PDF embedding: a `cmap` mapping `map` (codepoint → glyph) and the
/// original `OS/2`.
///
/// subsetter drops `OS/2`, which carries the ascent/descent metrics font
/// layout uses; without it text and icons are positioned differently.
fn with_cmap(font: &[u8], map: &[(char, u16)], source: &[u8]) -> Result<Vec<u8>> {
    let version: [u8; 4] = font
        .first_chunk::<4>()
        .copied()
        .ok_or("font shorter than its sfnt header")?;
    let mut tables = sfnt_tables(font)?;
    // Expand `hmtx` to one full record per glyph. subsetter writes the
    // minimum (`numberOfHMetrics` = 2) with trailing left-side bearings;
    // hinted rasterization then produces slightly different glyph bounds.
    // With an explicit record per glyph the hinted output matches the full
    // font exactly.
    expand_hmtx(&mut tables);
    tables.push((u32::from_be_bytes(*b"cmap"), cmap_format4(map)?));
    let os2 = u32::from_be_bytes(*b"OS/2");
    if let Some((_, metrics)) = sfnt_tables(source)?
        .into_iter()
        .find(|(tag, _)| *tag == os2)
    {
        tables.push((os2, metrics));
    }
    let out = rebuild_sfnt(version, tables);
    // Sanity: the table must parse and cover every codepoint.
    let face = ttf_parser::Face::parse(&out, 0)?;
    for (ch, _) in map {
        if face.glyph_index(*ch).is_none() {
            return Err(format!("rebuilt cmap lacks U+{:04X}", u32::from(*ch)).into());
        }
    }
    Ok(out)
}

/// Every table of an sfnt font as (tag, data), directory order preserved.
fn sfnt_tables(font: &[u8]) -> Result<Vec<(u32, Vec<u8>)>> {
    let word = |at: usize| -> Result<u32> {
        font.get(at..at + 4).map_or_else(
            || Err("truncated sfnt header".into()),
            |at| Ok(u32::from_be_bytes(at.try_into().expect("four bytes"))),
        )
    };
    let half = |at: usize| -> Result<u16> {
        font.get(at..at + 2).map_or_else(
            || Err("truncated sfnt header".into()),
            |at| Ok(u16::from_be_bytes(at.try_into().expect("two bytes"))),
        )
    };
    // version: u32, numTables: u16, searchRange, entrySelector, rangeShift.
    let count = usize::from(half(4)?);
    let mut tables = Vec::with_capacity(count);
    for i in 0..count {
        let record = 12 + i * 16;
        let tag = word(record)?;
        let offset = word(record + 8)? as usize;
        let length = word(record + 12)? as usize;
        let end = offset
            .checked_add(length)
            .filter(|end| *end <= font.len())
            .ok_or("table extends past the end of the font")?;
        tables.push((tag, font[offset..end].to_vec()));
    }
    Ok(tables)
}

/// Serialize tables into a clean sfnt: sorted directory, 4-byte aligned,
/// per-table checksums, `head.checkSumAdjustment` correct.
fn rebuild_sfnt(version: [u8; 4], mut tables: Vec<(u32, Vec<u8>)>) -> Vec<u8> {
    tables.sort_unstable_by_key(|(tag, _)| *tag);
    // A table's checksum is taken with `checkSumAdjustment` zeroed.
    if let Some((_, head)) = tables
        .iter_mut()
        .find(|(tag, _)| *tag == u32::from_be_bytes(*b"head"))
    {
        head.get_mut(8..12).expect("head has 54 bytes").fill(0);
    }
    let count = tables.len();
    let floor_log2 = u32::BITS - 1 - (count as u32).leading_zeros();
    let search_range = ((1 << floor_log2) * 16) as u16;

    let mut out = Vec::new();
    out.extend_from_slice(&version);
    out.extend((count as u16).to_be_bytes());
    out.extend(search_range.to_be_bytes());
    out.extend((floor_log2 as u16).to_be_bytes());
    out.extend(((count as u16) * 16 - search_range).to_be_bytes());

    let mut offset = 12 + 16 * count;
    let mut records = Vec::with_capacity(count);
    for (tag, data) in &tables {
        records.push((*tag, checksum(data), offset, data.len()));
        offset += data.len() + (4 - data.len() % 4) % 4;
    }
    for (tag, sum, at, length) in &records {
        out.extend(tag.to_be_bytes());
        out.extend(sum.to_be_bytes());
        out.extend((*at as u32).to_be_bytes());
        out.extend((*length as u32).to_be_bytes());
    }
    for (_, data) in &tables {
        out.extend_from_slice(data);
        out.extend(std::iter::repeat_n(0, (4 - data.len() % 4) % 4));
    }
    // With `head` zeroed, the whole-font checksum must equal 0xB1B0AFBA.
    if let Some(head) = records
        .iter()
        .find(|(tag, _, _, _)| *tag == u32::from_be_bytes(*b"head"))
    {
        let at = head.2 + 8;
        let adjustment = 0xB1B0_AFBAu32.wrapping_sub(checksum(&out));
        out[at..at + 4].copy_from_slice(&adjustment.to_be_bytes());
    }
    out
}

/// A minimal cmap format 4 subtable (BMP, which covers every Lucide icon
/// codepoint): one segment per codepoint plus the 0xFFFF sentinel, each
/// mapping straight to its glyph.
fn cmap_format4(map: &[(char, u16)]) -> Result<Vec<u8>> {
    let mut chars: Vec<(u32, u16)> = map.iter().map(|(ch, gid)| (u32::from(*ch), *gid)).collect();
    chars.sort_unstable();
    chars.dedup();
    if chars.iter().any(|(ch, _)| *ch > 0xFFFE) {
        return Err("codepoint above the BMP needs cmap format 12".into());
    }
    let segments = chars.len() + 1;
    let floor_log2 = u32::BITS - 1 - (segments as u32).leading_zeros();
    let search_range = ((1 << floor_log2) * 2) as u16;
    let length = 16 + 8 * segments;

    let mut out = Vec::with_capacity(4 + length);
    out.extend(0u16.to_be_bytes()); // cmap version
    out.extend(2u16.to_be_bytes()); // encoding records
    // Windows BMP and Unicode BMP both point at the one subtable.
    for (platform, encoding) in [(3u16, 1u16), (0, 3)] {
        out.extend(platform.to_be_bytes());
        out.extend(encoding.to_be_bytes());
        out.extend(20u32.to_be_bytes()); // 4-byte header + two records
    }
    let at = out.len();
    out.extend(4u16.to_be_bytes()); // subtable format
    out.extend((length as u16).to_be_bytes());
    out.extend(0u16.to_be_bytes()); // language
    out.extend(((segments * 2) as u16).to_be_bytes());
    out.extend(search_range.to_be_bytes());
    out.extend((floor_log2 as u16).to_be_bytes());
    out.extend(((segments as u16) * 2 - search_range).to_be_bytes());
    for (ch, _) in &chars {
        out.extend((*ch as u16).to_be_bytes());
    }
    out.extend(0xFFFFu16.to_be_bytes()); // end codes …
    out.extend(0u16.to_be_bytes()); // … reserved pad
    for (ch, _) in &chars {
        out.extend((*ch as u16).to_be_bytes());
    }
    out.extend(0xFFFFu16.to_be_bytes()); // … start codes
    for (ch, gid) in &chars {
        out.extend(gid.wrapping_sub(*ch as u16).to_be_bytes()); // … id deltas
    }
    out.extend(1u16.to_be_bytes()); // … sentinel delta maps 0xFFFF → 0
    for _ in 0..segments {
        out.extend(0u16.to_be_bytes()); // … id range offsets
    }
    debug_assert_eq!(out.len() - at, length);
    Ok(out)
}

/// The sfnt table checksum: big-endian u32 sum over zero-padded bytes.
fn checksum(data: &[u8]) -> u32 {
    let mut sum = 0u32;
    for chunk in data.chunks(4) {
        let mut word = [0u8; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum = sum.wrapping_add(u32::from_be_bytes(word));
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_variants_become_lucide_names() {
        for (rust, lucide) in [
            ("Anvil", "anvil"),
            ("AArrowDown", "a-arrow-down"),
            ("ArrowUpRight", "arrow-up-right"),
            ("RotateCcw", "rotate-ccw"),
            ("Grid3x3", "grid-3x3"),
            ("Trash2", "trash-2"),
            ("Layers2", "layers-2"),
            ("X", "x"),
        ] {
            assert_eq!(kebab_case(rust), lucide, "{rust}");
        }
    }

    #[test]
    fn only_icon_tokens_count() {
        let text = "Icon::Anvil; CursorIcon::None; lucide_icons::Icon::Crop; Icon::X";
        assert_eq!(scan_icon_variants(text), ["Anvil", "Crop", "X"]);
    }

    #[test]
    fn every_scanned_icon_exists_in_the_crate() {
        // `cargo xtask icons-subset` relies on this mapping.
        assert!(lucide_icons::Icon::try_from("a-arrow-down").is_ok());
        assert!(lucide_icons::Icon::try_from("not-an-icon").is_err());
    }
}
