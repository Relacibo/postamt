use lopdf::{Document, Object, Dictionary};
use sha2::{Sha256, Digest};
use std::fs;
use std::io::Read;
use std::path::Path;
use crate::error::{Error, Result};

pub fn compute_hash(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    hasher.update(&buffer);
    let result = hasher.finalize();
    Ok(hex::encode(result))
}

pub fn extract_matrix_codes(path: &Path, grid_cols: usize, grid_rows_max: usize) -> Result<Vec<String>> {
    let doc = Document::load(path)?;
    let mut matrix_codes = Vec::new();
    
    let pages = doc.get_pages();
    if pages.len() != 1 {
        return Err(Error::Custom("Only single-page PDFs are supported".to_string()));
    }
    
    let page_num = *pages.keys().next().unwrap();
    let text = doc.extract_text(&[page_num])?;
    
    let re = regex::Regex::new(r"A0 [0-9A-F]{4} [0-9A-F]{4} [0-9A-F]{2} [0-9A-F]{4} [0-9A-F]{4}").unwrap();
    
    for cap in re.find_iter(&text) {
        matrix_codes.push(cap.as_str().to_string());
    }
    
    let max_stamps = grid_cols * grid_rows_max;
    if matrix_codes.len() > max_stamps {
        return Err(Error::Custom(format!(
            "Too many stamps found: {} (max: {})",
            matrix_codes.len(),
            max_stamps
        )));
    }
    
    Ok(matrix_codes)
}

pub fn extract_stamp(source_path: &Path, stamp_index: usize, grid_cols: usize) -> Result<Vec<u8>> {
    let doc = Document::load(source_path)?;
    
    let row = stamp_index / grid_cols;
    let col = stamp_index % grid_cols;
    
    let stamp_width = 148.0;
    let stamp_height = 105.0;
    let x_offset = col as f32 * stamp_width;
    let y_offset = 841.890 - (row as f32 + 1.0) * stamp_height;
    
    let pages = doc.get_pages();
    let page_id = *pages.keys().next().unwrap();
    
    let mut new_doc = Document::with_version("1.7");
    
    let page_dict = doc.get_object(*pages.get(&page_id).unwrap())?.as_dict()?;
    let mut new_page_dict = page_dict.clone();
    
    let crop_box = vec![
        Object::Real(x_offset),
        Object::Real(y_offset),
        Object::Real(x_offset + stamp_width),
        Object::Real(y_offset + stamp_height),
    ];
    new_page_dict.set("CropBox", Object::Array(crop_box));
    new_page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0),
        Object::Real(0.0),
        Object::Real(stamp_width),
        Object::Real(stamp_height),
    ]));
    
    if let Ok(resources) = page_dict.get(b"Resources") {
        new_page_dict.set("Resources", resources.clone());
    }
    if let Ok(contents) = page_dict.get(b"Contents") {
        new_page_dict.set("Contents", contents.clone());
    }
    
    let new_page_id = new_doc.add_object(Object::Dictionary(new_page_dict));
    let pages_id = new_doc.new_object_id();
    
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", Object::Name(b"Pages".to_vec()));
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![Object::Reference(new_page_id)]));
    
    new_doc.objects.insert(pages_id, Object::Dictionary(pages_dict));
    
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", Object::Name(b"Catalog".to_vec()));
    catalog_dict.set("Pages", Object::Reference(pages_id));
    
    let catalog_id = new_doc.add_object(Object::Dictionary(catalog_dict));
    new_doc.trailer.set("Root", Object::Reference(catalog_id));
    
    let mut buffer = Vec::new();
    new_doc.save_to(&mut buffer)?;
    
    Ok(buffer)
}

pub fn create_envelope(profile_width: f32, profile_height: f32, 
                       _offset_x: f32, _offset_y: f32, 
                       _stamp_pdf: &[u8]) -> Result<Vec<u8>> {
    use printpdf::*;
    
    let doc = PdfDocument::new("Envelope");
    let _page = PdfPage::new(Mm(profile_width), Mm(profile_height), vec![]);
    
    let mut warnings = vec![];
    let opts = PdfSaveOptions::default();
    let buffer = doc.save(&opts, &mut warnings);
    
    Ok(buffer)
}
