use lopdf::{Dictionary, Document, Object, Stream};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Choose random template
    let templates = [
        "assets/pdfs/example-pdfs/TestPrint.pdf",
        "assets/pdfs/example-pdfs/TestPrint-full.pdf",
    ];

    // Generate seed for random template selection
    let seed = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let mut lcg = SimpleLcg::new(seed);

    let template_path = templates[lcg.next_usize() % templates.len()];

    if !Path::new(template_path).exists() {
        eprintln!("Error: Template not found: {}", template_path);
        std::process::exit(1);
    }

    // Generate random matrix code (format: XXXX XXXX XXXX XX XXXX XXXXX)
    let matrix = generate_random_matrix(&mut lcg);

    // Load template and replace all X with random characters
    let mut pdf = Document::load(template_path)?;

    // Get all pages and replace X in each
    let page_ids: Vec<_> = pdf.get_pages().keys().cloned().collect();

    for page_id in page_ids {
        if let Err(e) = replace_x_in_page(&mut pdf, page_id, &matrix) {
            eprintln!("Warning: Failed to process page {}: {}", page_id, e);
        }
    }

    // Save output
    let output_filename = format!("generated-{}.pdf", matrix.replace(" ", "_"));
    let mut buffer = Vec::new();
    pdf.save_to(&mut buffer)?;
    std::fs::write(&output_filename, buffer)?;

    println!("Generated: {} (matrix: {})", output_filename, matrix);
    Ok(())
}

fn replace_x_in_page(
    pdf: &mut Document,
    page_id: u32,
    matrix: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let page_ref = pdf.get_pages()[&page_id];

    let page_obj = pdf.get_object(page_ref)?;
    let page_dict = page_obj.as_dict()?;
    let Object::Reference(contents_id) = page_dict.get(b"Contents")? else {
        return Err("Contents is not a reference".into());
    };

    let stream_obj = pdf.get_object(*contents_id)?;
    let Object::Stream(stream) = stream_obj else {
        return Err("Contents is not a stream".into());
    };

    let content = stream
        .decompressed_content()
        .unwrap_or_else(|_| stream.content.clone());

    // Replace all "X" sequences with matrix code
    let content_str = String::from_utf8_lossy(&content);
    let matrix_chars: Vec<char> = matrix.chars().filter(|c| *c != ' ').collect();

    let mut result = String::new();
    let mut i = 0;
    let chars: Vec<char> = content_str.chars().collect();
    let mut matrix_idx = 0;

    while i < chars.len() {
        if chars[i] == 'X' {
            // Found start of X sequence
            while i < chars.len() && chars[i] == 'X' {
                result.push(matrix_chars[matrix_idx % matrix_chars.len()]);
                matrix_idx += 1;
                i += 1;
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    // Create new stream
    let new_stream = Object::Stream(Stream::new(Dictionary::new(), result.into_bytes()));
    let new_stream_id = pdf.add_object(new_stream);

    // Update page contents
    let page_obj_mut = pdf.get_object_mut(page_ref)?;
    let page_dict_mut = page_obj_mut.as_dict_mut()?;
    page_dict_mut.set("Contents", Object::Reference(new_stream_id));

    Ok(())
}

// Simple linear congruential generator
struct SimpleLcg {
    state: u64,
}

impl SimpleLcg {
    fn new(seed: u64) -> Self {
        SimpleLcg { state: seed }
    }

    fn next(&mut self) -> u64 {
        const A: u64 = 1103515245;
        const C: u64 = 12345;
        const M: u64 = 2u64.pow(31);

        self.state = (A.wrapping_mul(self.state).wrapping_add(C)) % M;
        self.state
    }

    fn next_usize(&mut self) -> usize {
        self.next() as usize
    }
}

fn generate_random_matrix(rng: &mut SimpleLcg) -> String {
    // Format: XXXX XXXX XXXX XX XXXX XXXXX
    const CHARSET: &[u8] = b"0123456789ABCDEF";

    let mut result = String::new();
    let segments = [4, 4, 4, 2, 4, 5];

    for (i, &len) in segments.iter().enumerate() {
        if i > 0 {
            result.push(' ');
        }
        for _ in 0..len {
            let idx = rng.next_usize() % CHARSET.len();
            result.push(CHARSET[idx] as char);
        }
    }

    result
}
