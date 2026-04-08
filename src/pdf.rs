use crate::error::{Error, Result};
use lopdf::{Dictionary, Document, Object};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::Path;

// Grid layout constants for Deutsche Post stamp sheets
pub const GRID_COLS: usize = 4;
pub const GRID_ROWS_MAX: usize = 8;

pub fn compute_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

pub fn extract_matrix_codes(path: &Path) -> Result<Vec<String>> {
    let doc = Document::load(path)?;
    let mut matrix_codes = Vec::new();

    let pages = doc.get_pages();
    if pages.len() != 1 {
        return Err(Error::Custom(
            "Only single-page PDFs are supported".to_string(),
        ));
    }

    let page_num = *pages.keys().next().unwrap();
    let text = doc.extract_text(&[page_num])?;

    let re = regex::Regex::new(r"A0 [0-9A-F]{4} [0-9A-F]{4} [0-9A-F]{2} [0-9A-F]{4} [0-9A-F]{4}")
        .unwrap();

    for cap in re.find_iter(&text) {
        matrix_codes.push(cap.as_str().to_string());
    }

    let max_stamps = GRID_COLS * GRID_ROWS_MAX;
    if matrix_codes.len() > max_stamps {
        return Err(Error::Custom(format!(
            "Too many stamps found: {} (max: {})",
            matrix_codes.len(),
            max_stamps
        )));
    }

    Ok(matrix_codes)
}

pub fn extract_stamp(source_path: &Path, stamp_index: usize) -> Result<Vec<u8>> {
    let mut doc = Document::load(source_path)?;

    let row = stamp_index / GRID_COLS;
    let col = stamp_index % GRID_COLS;

    // Exact cross positions measured from TestPrint-full.pdf at 72 DPI
    // Crosses mark the corners of stamps, stamps are BETWEEN crosses
    let cross_x = [60.926, 174.787, 287.650, 400.512, 514.374];
    let cross_y_from_top = [
        16.998, 116.985, 215.972, 315.958, 414.945, 514.932, 613.919, 713.906, 812.893,
    ];

    // Stamp is between cross[i] and cross[i+1]
    if col >= cross_x.len() - 1 {
        return Err(Error::Custom("Column index out of range".to_string()));
    }
    if row >= cross_y_from_top.len() - 1 {
        return Err(Error::Custom("Row index out of range".to_string()));
    }

    let x_min = cross_x[col];
    let x_max = cross_x[col + 1];
    let y_from_top_min = cross_y_from_top[row];
    let y_from_top_max = cross_y_from_top[row + 1];

    // Convert Y to PDF coordinates (from bottom)
    let page_height = 841.889;
    let y_min = page_height - y_from_top_max;
    let y_max = page_height - y_from_top_min;

    // Set CropBox
    let pages = doc.get_pages();
    let page_id = *pages.keys().next().unwrap();
    let page_ref = *pages.get(&page_id).unwrap();

    if let Ok(page_dict) = doc.get_object_mut(page_ref).and_then(|o| o.as_dict_mut()) {
        page_dict.set(
            "CropBox",
            Object::Array(vec![
                Object::Real(x_min),
                Object::Real(y_min),
                Object::Real(x_max),
                Object::Real(y_max),
            ]),
        );
    }

    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)?;

    Ok(buffer)
}

pub fn create_envelope(
    profile_width: f32,
    profile_height: f32,
    offset_x: f32,
    offset_y: f32,
    stamp_pdf: &[u8],
) -> Result<Vec<u8>> {
    let mut doc = Document::load_mem(stamp_pdf)?;

    // Convert mm to points
    let page_width = profile_width * 2.834_645_7;
    let page_height = profile_height * 2.834_645_7;
    let offset_x_pt = offset_x * 2.834_645_7;
    let offset_y_pt = offset_y * 2.834_645_7;

    let pages = doc.get_pages();
    let page_id = *pages.keys().next().unwrap();
    let page_ref = *pages.get(&page_id).unwrap();

    // Get CropBox (where stamp currently is in the A4 page)
    let (crop_x_min, crop_y_min) =
        if let Ok(page_dict) = doc.get_object(page_ref).and_then(|o| o.as_dict()) {
            if let Ok(Object::Array(crop_box)) = page_dict.get(b"CropBox") {
                let x_min = crop_box[0].as_f32().unwrap_or(0.0);
                let y_min = crop_box[1].as_f32().unwrap_or(0.0);
                (x_min, y_min)
            } else {
                (0.0, 0.0)
            }
        } else {
            (0.0, 0.0)
        };

    // Get existing content stream
    let (_, existing_content) =
        if let Ok(page_dict) = doc.get_object(page_ref).and_then(|o| o.as_dict()) {
            if let Ok(Object::Reference(id)) = page_dict.get(b"Contents") {
                let content = if let Ok(Object::Stream(stream)) = doc.get_object(*id) {
                    stream
                        .decompressed_content()
                        .unwrap_or_else(|_| stream.content.clone())
                } else {
                    vec![]
                };
                (Some(*id), content)
            } else {
                (None, vec![])
            }
        } else {
            (None, vec![])
        };

    // Create transformation to move stamp from its current position to offset
    // Current position: crop_x_min, crop_y_min (in A4 coordinates, Y from bottom)
    // Desired position: offset_x_pt, offset_y_pt
    // Note: offset_y is interpreted as distance from TOP of envelope (standard for stamps)
    // So we need: envelope_height - offset_y_pt (to convert to "from bottom")
    let target_y_pt = page_height - offset_y_pt - (113.861 * 0.88); // Adjust for stamp height (~100pts)

    let mut new_content = format!(
        "q\n1 0 0 1 {} {} cm\n",
        offset_x_pt - crop_x_min,
        target_y_pt - crop_y_min
    )
    .into_bytes();
    new_content.extend_from_slice(&existing_content);
    new_content.extend_from_slice(b"\nQ\n");

    // Create new stream
    let new_stream = Object::Stream(lopdf::Stream::new(Dictionary::new(), new_content));
    let new_contents_id = doc.add_object(new_stream);

    // Modify page
    if let Ok(page_dict) = doc.get_object_mut(page_ref).and_then(|o| o.as_dict_mut()) {
        // Set MediaBox to envelope size
        page_dict.set(
            "MediaBox",
            Object::Array(vec![
                Object::Real(0.0),
                Object::Real(0.0),
                Object::Real(page_width),
                Object::Real(page_height),
            ]),
        );

        // Remove CropBox - show the whole envelope
        page_dict.remove(b"CropBox");

        // Set new contents
        page_dict.set("Contents", Object::Reference(new_contents_id));
    }

    let mut buffer = Vec::new();
    doc.save_to(&mut buffer)?;

    Ok(buffer)
}
