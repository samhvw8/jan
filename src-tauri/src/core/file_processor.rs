use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessedFile {
    pub content: String,
    pub metadata: FileMetadata,
    pub processing_time: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileMetadata {
    pub name: String,
    pub size: usize,
    pub file_type: String,
    pub encoding: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum FileProcessingError {
    #[error("File not found: {0}")]
    FileNotFound(String),
    #[error("Unsupported file type: {0}")]
    UnsupportedFileType(String),
    #[error("File too large: {size} bytes (max: {max} bytes)")]
    FileTooLarge { size: u64, max: u64 },
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("UTF-8 conversion error: {0}")]
    Utf8Error(#[from] std::string::FromUtf8Error),
    #[error("PDF processing error: {0}")]
    PdfError(String),
    #[error("DOCX processing error: {0}")]
    DocxError(String),
    #[error("CSV processing error: {0}")]
    CsvError(#[from] csv::Error),
    #[error("Path traversal attempt detected")]
    PathTraversal,
    #[error("Memory limit exceeded")]
    MemoryLimitExceeded,
}

pub type Result<T> = std::result::Result<T, FileProcessingError>;

// File size limits in bytes
const MAX_TEXT_FILE_SIZE: u64 = 5 * 1024 * 1024; // 5MB
const MAX_CSV_FILE_SIZE: u64 = 10 * 1024 * 1024; // 10MB
const MAX_PDF_FILE_SIZE: u64 = 20 * 1024 * 1024; // 20MB
const MAX_DOCX_FILE_SIZE: u64 = 15 * 1024 * 1024; // 15MB

// Memory usage limit for processing
const MAX_MEMORY_USAGE: usize = 50 * 1024 * 1024; // 50MB

pub struct FileProcessor;

impl FileProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn process_file<P: AsRef<Path>>(file_path: P) -> Result<ProcessedFile> {
        let start_time = Instant::now();
        let path = file_path.as_ref();
        
        log::info!("Starting file processing for: {:?}", path);
        log::debug!("File path components: {:?}", path.components().collect::<Vec<_>>());

        // Security check: prevent path traversal
        if Self::has_path_traversal(path) {
            log::error!("Path traversal attempt detected for: {:?}", path);
            return Err(FileProcessingError::PathTraversal);
        }

        // Check if file exists
        if !path.exists() {
            log::error!("File does not exist: {:?}", path);
            return Err(FileProcessingError::FileNotFound(
                path.to_string_lossy().to_string(),
            ));
        }

        // Get file metadata
        let file_metadata = match fs::metadata(path) {
            Ok(metadata) => {
                log::debug!("File metadata retrieved successfully");
                metadata
            }
            Err(e) => {
                log::error!("Failed to get file metadata for {:?}: {}", path, e);
                return Err(FileProcessingError::IoError(e));
            }
        };
        
        let file_size = file_metadata.len();
        log::debug!("File size: {} bytes", file_size);
        
        let file_name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string();
        log::debug!("File name: {}", file_name);

        let file_extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        log::debug!("File extension: {}", file_extension);

        // Validate file size based on type
        if let Err(e) = Self::validate_file_size(file_size, &file_extension) {
            log::error!("File size validation failed: {}", e);
            return Err(e);
        }
        log::debug!("File size validation passed");

        // Process file based on extension
        log::info!("Processing file with extension: {}", file_extension);
        let (content, encoding) = match file_extension.as_str() {
            "txt" | "md" | "json" => {
                log::debug!("Processing as text file");
                Self::process_text_file(path)?
            }
            "csv" => {
                log::debug!("Processing as CSV file");
                Self::process_csv_file(path)?
            }
            "pdf" => {
                log::debug!("Processing as PDF file");
                Self::process_pdf_file(path)?
            }
            "docx" => {
                log::debug!("Processing as DOCX file");
                Self::process_docx_file(path)?
            }
            _ => {
                log::error!("Unsupported file type: {}", file_extension);
                return Err(FileProcessingError::UnsupportedFileType(
                    file_extension,
                ));
            }
        };

        log::debug!("Content extracted, length: {} characters", content.len());
        log::debug!("Encoding detected: {:?}", encoding);

        // Check memory usage
        if content.len() > MAX_MEMORY_USAGE {
            log::error!("Memory limit exceeded: {} > {}", content.len(), MAX_MEMORY_USAGE);
            return Err(FileProcessingError::MemoryLimitExceeded);
        }

        let processing_time = start_time.elapsed().as_millis() as u64;
        log::info!("File processing completed in {} ms", processing_time);

        let processed_file = ProcessedFile {
            content,
            metadata: FileMetadata {
                name: file_name,
                size: file_size as usize,
                file_type: file_extension,
                encoding,
            },
            processing_time,
        };

        log::debug!("ProcessedFile created successfully");
        Ok(processed_file)
    }

    fn has_path_traversal(path: &Path) -> bool {
        path.components().any(|component| {
            matches!(
                component,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
    }

    fn validate_file_size(size: u64, file_type: &str) -> Result<()> {
        let max_size = match file_type {
            "txt" | "md" | "json" => MAX_TEXT_FILE_SIZE,
            "csv" => MAX_CSV_FILE_SIZE,
            "pdf" => MAX_PDF_FILE_SIZE,
            "docx" => MAX_DOCX_FILE_SIZE,
            _ => MAX_TEXT_FILE_SIZE, // Default to text file limit
        };

        if size > max_size {
            return Err(FileProcessingError::FileTooLarge {
                size,
                max: max_size,
            });
        }

        Ok(())
    }

    fn process_text_file(path: &Path) -> Result<(String, Option<String>)> {
        log::debug!("Reading text file: {:?}", path);
        let bytes = match fs::read(path) {
            Ok(bytes) => {
                log::debug!("Successfully read {} bytes from text file", bytes.len());
                bytes
            }
            Err(e) => {
                log::error!("Failed to read text file {:?}: {}", path, e);
                return Err(FileProcessingError::IoError(e));
            }
        };
        
        // Try UTF-8 first
        log::debug!("Attempting UTF-8 decoding");
        match String::from_utf8(bytes.clone()) {
            Ok(content) => {
                log::debug!("Successfully decoded as UTF-8, content length: {}", content.len());
                Ok((content, Some("UTF-8".to_string())))
            }
            Err(e) => {
                log::debug!("UTF-8 decoding failed: {}, trying encoding detection", e);
                // Try to detect encoding and convert
                let (content, encoding, _) = encoding_rs::UTF_8.decode(&bytes);
                if encoding == encoding_rs::UTF_8 {
                    log::debug!("Encoding detection confirmed UTF-8");
                    Ok((content.into_owned(), Some("UTF-8".to_string())))
                } else {
                    log::debug!("Trying alternative encodings");
                    // Try common encodings
                    for encoding in &[encoding_rs::WINDOWS_1252, encoding_rs::ISO_8859_2] {
                        log::debug!("Trying encoding: {}", encoding.name());
                        let (decoded, _, had_errors) = encoding.decode(&bytes);
                        if !had_errors {
                            log::debug!("Successfully decoded with encoding: {}", encoding.name());
                            return Ok((decoded.into_owned(), Some(encoding.name().to_string())));
                        }
                    }
                    
                    // Fallback: use UTF-8 with replacement characters
                    log::warn!("All encoding attempts failed, using UTF-8 with replacement characters");
                    let content = String::from_utf8_lossy(&bytes);
                    Ok((content.into_owned(), Some("UTF-8 (with replacements)".to_string())))
                }
            }
        }
    }

    fn process_csv_file(path: &Path) -> Result<(String, Option<String>)> {
        log::debug!("Processing CSV file: {:?}", path);
        let mut reader = match csv::Reader::from_path(path) {
            Ok(reader) => {
                log::debug!("CSV reader created successfully");
                reader
            }
            Err(e) => {
                log::error!("Failed to create CSV reader for {:?}: {}", path, e);
                return Err(FileProcessingError::CsvError(e));
            }
        };
        let mut content = String::new();
        
        // Get headers
        log::debug!("Reading CSV headers");
        let headers = match reader.headers() {
            Ok(headers) => {
                let header_vec: Vec<&str> = headers.iter().collect();
                log::debug!("CSV headers: {:?}", header_vec);
                headers.clone()
            }
            Err(e) => {
                log::error!("Failed to read CSV headers: {}", e);
                return Err(FileProcessingError::CsvError(e));
            }
        };
        content.push_str(&format!("Headers: {}\n\n", headers.iter().collect::<Vec<_>>().join(", ")));
        
        // Process records
        log::debug!("Processing CSV records");
        let mut record_count = 0;
        for result in reader.records() {
            match result {
                Ok(record) => {
                    if record_count < 100 { // Limit preview to first 100 records
                        content.push_str(&format!("Row {}: {}\n", record_count + 1,
                            record.iter().collect::<Vec<_>>().join(", ")));
                    }
                    record_count += 1;
                    
                    if record_count % 1000 == 0 {
                        log::debug!("Processed {} CSV records", record_count);
                    }
                }
                Err(e) => {
                    log::error!("Error reading CSV record {}: {}", record_count + 1, e);
                    return Err(FileProcessingError::CsvError(e));
                }
            }
        }
        
        log::debug!("CSV processing completed. Total records: {}", record_count);
        
        if record_count > 100 {
            content.push_str(&format!("\n... and {} more records", record_count - 100));
        }
        
        content.push_str(&format!("\n\nTotal records: {}", record_count));
        
        Ok((content, Some("UTF-8".to_string())))
    }

    fn process_pdf_file(path: &Path) -> Result<(String, Option<String>)> {
        log::debug!("Processing PDF file: {:?}", path);
        match pdf_extract::extract_text(path) {
            Ok(content) => {
                log::debug!("PDF text extraction successful, content length: {}", content.len());
                if content.trim().is_empty() {
                    log::warn!("PDF file contains no extractable text content");
                    Ok(("PDF file processed but no text content was extracted. This may be a scanned document or contain only images.".to_string(), None))
                } else {
                    log::debug!("PDF text extraction completed successfully");
                    Ok((content, None))
                }
            }
            Err(e) => {
                log::error!("Failed to extract text from PDF {:?}: {}", path, e);
                Err(FileProcessingError::PdfError(format!("Failed to extract text from PDF: {}", e)))
            }
        }
    }

    fn process_docx_file(path: &Path) -> Result<(String, Option<String>)> {
        log::debug!("Processing DOCX file: {:?}", path);
        let bytes = match fs::read(path) {
            Ok(bytes) => {
                log::debug!("Successfully read {} bytes from DOCX file", bytes.len());
                bytes
            }
            Err(e) => {
                log::error!("Failed to read DOCX file {:?}: {}", path, e);
                return Err(FileProcessingError::IoError(e));
            }
        };
        
        match docx_rs::read_docx(&bytes) {
            Ok(docx) => {
                log::debug!("DOCX file parsed successfully");
                let mut content = String::new();
                let mut paragraph_count = 0;
                let mut text_runs = 0;
                
                // Extract text from paragraphs
                for child in docx.document.children {
                    match child {
                        docx_rs::DocumentChild::Paragraph(paragraph) => {
                            paragraph_count += 1;
                            for run in paragraph.children {
                                match run {
                                    docx_rs::ParagraphChild::Run(run) => {
                                        for run_child in run.children {
                                            if let docx_rs::RunChild::Text(text) = run_child {
                                                content.push_str(&text.text);
                                                text_runs += 1;
                                            }
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            content.push('\n');
                        }
                        _ => {}
                    }
                }
                
                log::debug!("DOCX processing completed: {} paragraphs, {} text runs, {} characters",
                           paragraph_count, text_runs, content.len());
                
                if content.trim().is_empty() {
                    log::warn!("DOCX file contains no extractable text content");
                    Ok(("DOCX file processed but no text content was found.".to_string(), None))
                } else {
                    log::debug!("DOCX text extraction completed successfully");
                    Ok((content, Some("UTF-8".to_string())))
                }
            }
            Err(e) => {
                log::error!("Failed to parse DOCX file {:?}: {}", path, e);
                Err(FileProcessingError::DocxError(format!("Failed to read DOCX file: {}", e)))
            }
        }
    }

    pub fn get_supported_extensions() -> Vec<&'static str> {
        vec!["txt", "md", "json", "csv", "pdf", "docx"]
    }

    pub fn is_supported_file<P: AsRef<Path>>(file_path: P) -> bool {
        let path = file_path.as_ref();
        if let Some(extension) = path.extension().and_then(|ext| ext.to_str()) {
            Self::get_supported_extensions().contains(&extension.to_lowercase().as_str())
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::tempdir;

    #[test]
    fn test_path_traversal_detection() {
        assert!(FileProcessor::has_path_traversal(Path::new("../test.txt")));
        assert!(FileProcessor::has_path_traversal(Path::new("./test.txt")));
        assert!(FileProcessor::has_path_traversal(Path::new("folder/../test.txt")));
        assert!(!FileProcessor::has_path_traversal(Path::new("test.txt")));
        assert!(!FileProcessor::has_path_traversal(Path::new("folder/test.txt")));
    }

    #[test]
    fn test_supported_extensions() {
        assert!(FileProcessor::is_supported_file("test.txt"));
        assert!(FileProcessor::is_supported_file("test.PDF"));
        assert!(!FileProcessor::is_supported_file("test.exe"));
        assert!(!FileProcessor::is_supported_file("test"));
    }

    #[test]
    fn test_process_text_file() -> Result<()> {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.txt");
        let mut file = fs::File::create(&file_path)?;
        writeln!(file, "Hello, World!")?;

        let result = FileProcessor::process_file(&file_path)?;
        assert_eq!(result.content.trim(), "Hello, World!");
        assert_eq!(result.metadata.file_type, "txt");
        assert_eq!(result.metadata.encoding, Some("UTF-8".to_string()));

        Ok(())
    }

    #[test]
    fn test_file_not_found() {
        let result = FileProcessor::process_file("nonexistent.txt");
        assert!(matches!(result, Err(FileProcessingError::FileNotFound(_))));
    }

    #[test]
    fn test_unsupported_file_type() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.exe");
        fs::File::create(&file_path).unwrap();

        let result = FileProcessor::process_file(&file_path);
        assert!(matches!(result, Err(FileProcessingError::UnsupportedFileType(_))));
    }

    #[test]
    fn test_csv_processing() -> Result<()> {
        let dir = tempdir().unwrap();
        let csv_file_path = dir.path().join("test.csv");
        let mut csv_file = fs::File::create(&csv_file_path)?;
        writeln!(csv_file, "name,age,city")?;
        writeln!(csv_file, "John,30,New York")?;
        writeln!(csv_file, "Jane,25,Los Angeles")?;

        let result = FileProcessor::process_file(&csv_file_path)?;
        assert!(result.content.contains("Headers:"));
        assert!(result.content.contains("name, age, city"));
        assert!(result.content.contains("John,30,New York"));
        assert!(result.content.contains("Total records: 2"));
        assert_eq!(result.metadata.file_type, "csv");

        Ok(())
    }
}