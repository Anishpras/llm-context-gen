// main.rs
use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::env;
use std::collections::HashSet;
use structopt::StructOpt;
use ignore::WalkBuilder;

#[derive(Debug, StructOpt)]
#[structopt(name = "llm-context-gen", about = "Generate text files for LLM context from source code")]
struct Opt {
    /// The directory to process
    #[structopt(short, long, default_value = ".")]
    dir: String,

    /// The output directory
    #[structopt(short, long, default_value = "llm-context")]
    output: String,

    /// Additional directories to ignore (comma-separated)
    #[structopt(short, long, default_value = "")]
    ignore: String,
    
    /// Maximum number of files to process
    #[structopt(short, long, default_value = "2000")]
    max_files: usize,
    
    /// Maximum file size to process in bytes
    #[structopt(short, long, default_value = "500000")]
    max_size: u64,
    
    /// Maximum directory depth
    #[structopt(long, default_value = "8")]
    max_depth: usize,
}

// Safer indentation function that doesn't use repeat
fn get_indent(depth: usize) -> String {
    let max_indent = 10; // Maximum safe indent level
    let safe_depth = depth.min(max_indent);
    
    let mut result = String::with_capacity(safe_depth * 4); // Pre-allocate space for efficiency
    for _ in 0..safe_depth {
        result.push_str("│   ");
    }
    
    result
}

fn main() -> io::Result<()> {
    let opt = Opt::from_args();
    
    // Create output directory
    let output_dir = Path::new(&opt.output);
    fs::create_dir_all(output_dir)?;
    
    // Create file-tree.txt
    let file_tree_path = output_dir.join("file-tree.txt");
    let mut file_tree = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(file_tree_path)?;
    
    // Default directories to ignore
    let mut default_ignores = HashSet::new();
    default_ignores.insert("node_modules".to_string());
    default_ignores.insert("target".to_string());
    default_ignores.insert("dist".to_string());
    default_ignores.insert("build".to_string());
    default_ignores.insert(".git".to_string());
    default_ignores.insert(".idea".to_string());
    default_ignores.insert(".vscode".to_string());
    default_ignores.insert("__pycache__".to_string());
    
    // Next.js specific directories
    default_ignores.insert(".next".to_string());
    default_ignores.insert("out".to_string());
    default_ignores.insert("coverage".to_string());
    default_ignores.insert(".vercel".to_string());
    default_ignores.insert(".turbo".to_string());
    
    // Add user-specified ignores
    if !opt.ignore.is_empty() {
        for ignore in opt.ignore.split(',') {
            default_ignores.insert(ignore.trim().to_string());
        }
    }
    
    println!("Processing directory: {}", opt.dir);
    println!("Ignoring directories: {:?}", default_ignores);
    println!("Maximum files: {}", opt.max_files);
    println!("Maximum file size: {} bytes", opt.max_size);
    println!("Maximum depth: {}", opt.max_depth);
    
    // Set up a custom walker with limits
    let walker = WalkBuilder::new(&opt.dir)
        .hidden(false) // Don't skip hidden files by default
        .git_global(true) // Use global gitignore
        .git_ignore(true) // Use .gitignore
        .max_depth(Some(opt.max_depth)) // Limit directory depth
        .max_filesize(Some(opt.max_size)) // Skip files larger than specified size
        .build();
    
    // Initialize file tree string for the root directory
    writeln!(file_tree, ".")?;
    
    // Count processed files to prevent excessive processing
    let mut file_count = 0;
    let max_files = opt.max_files; // Use user-specified limit
    
    for result in walker {
        if file_count >= max_files {
            writeln!(file_tree, "\n[Maximum file limit reached ({}). Some files were skipped.]", max_files)?;
            println!("Maximum file limit reached ({}). Some files were skipped.", max_files);
            break;
        }
        
        match result {
            Ok(entry) => {
                let path = entry.path();
                
                // Skip the output directory itself
                if path.starts_with(output_dir) {
                    continue;
                }
                
                // Skip directories in our default ignore list
                let skip = path.components().any(|comp| {
                    if let Some(name) = comp.as_os_str().to_str() {
                        default_ignores.contains(name)
                    } else {
                        false
                    }
                });
                
                if skip {
                    continue;
                }
                
                // Use a safe way to get relative path
                let relative_path = match path.strip_prefix(&opt.dir) {
                    Ok(rel_path) => rel_path,
                    Err(_) => {
                        // If we can't get a relative path, just use the file name
                        if let Some(file_name) = path.file_name() {
                            Path::new(file_name)
                        } else {
                            continue; // Skip if we can't determine a path
                        }
                    }
                };
                
                // Add to file tree (with safety checks)
                if path.is_dir() {
                    // Limit nesting level for indentation to prevent overflow
                    let component_count = relative_path.components().count();
                    if component_count > 20 {
                        writeln!(file_tree, "[Deeply nested directory skipped]")?;
                        continue;
                    }
                    
                    // Use our safe indent function
                    let indent = get_indent(component_count.saturating_sub(1));
                    let dir_name = relative_path.file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    
                    writeln!(file_tree, "{}├── {}/", indent, dir_name)?;
                } else if path.is_file() {
                    process_file(path, relative_path, output_dir, &mut file_tree)?;
                    file_count += 1;
                    
                    if file_count % 100 == 0 {
                        println!("Processed {} files...", file_count);
                    }
                }
            }
            Err(err) => {
                eprintln!("Error: {}", err);
            }
        }
    }
    
    println!("Context files generated in: {}", output_dir.display());
    println!("Total files processed: {}", file_count);
    Ok(())
}

fn process_file(
    path: &Path,
    relative_path: &Path,
    output_dir: &Path,
    file_tree: &mut File,
) -> io::Result<()> {
    // Safety check for path length
    if relative_path.to_string_lossy().len() > 200 {
        let indent = get_indent(relative_path.components().count().saturating_sub(1));
        writeln!(file_tree, "{}├── ... (skipped - path too long)", indent)?;
        return Ok(());
    }

    // Skip binary files and very large files
    if is_binary_file(path)? || is_too_large(path)? {
        let indent = get_indent(relative_path.components().count().saturating_sub(1));
        writeln!(file_tree, "{}├── {} (skipped - binary or too large)", 
            indent, 
            relative_path.file_name().unwrap_or_default().to_string_lossy())?;
        return Ok(());
    }
    
    // Read file content - with proper error handling
    let mut content = String::new();
    match File::open(path) {
        Ok(mut file) => {
            if let Err(e) = file.read_to_string(&mut content) {
                eprintln!("Error reading file {}: {}", path.display(), e);
                let indent = get_indent(relative_path.components().count().saturating_sub(1));
                writeln!(file_tree, "{}├── {} (skipped - error reading)", 
                    indent, 
                    relative_path.file_name().unwrap_or_default().to_string_lossy())?;
                return Ok(());
            }
        },
        Err(e) => {
            eprintln!("Error opening file {}: {}", path.display(), e);
            return Ok(());
        }
    }
    
    // Create a safe filename for the output
    let safe_filename = sanitize_filename(&relative_path.to_string_lossy());
    let output_file_path = output_dir.join(format!("{}.txt", safe_filename));
    
    // Create output file with error handling
    let mut output_file = match File::create(&output_file_path) {
        Ok(file) => file,
        Err(e) => {
            eprintln!("Error creating output file {}: {}", output_file_path.display(), e);
            return Ok(());
        }
    };
    
    // Write file name with extension
    let file_name = path.file_name().unwrap_or_default().to_string_lossy();
    if let Err(e) = writeln!(output_file, "{}", file_name) {
        eprintln!("Error writing to output file: {}", e);
        return Ok(());
    }
    
    if let Err(e) = writeln!(output_file) {
        eprintln!("Error writing to output file: {}", e);
        return Ok(());
    }
    
    // Write content with error handling
    if let Err(e) = write!(output_file, "{}", content) {
        eprintln!("Error writing content to output file: {}", e);
    }
    
    // Add to file tree
    let indent = get_indent(relative_path.components().count().saturating_sub(1));
    writeln!(file_tree, "{}├── {}", 
        indent, 
        relative_path.file_name().unwrap_or_default().to_string_lossy())?;
    
    Ok(())
}

fn sanitize_filename(path: &str) -> String {
    // Replace path separators and limit length
    path.replace("/", "_")
        .replace("\\", "_")
        .chars()
        .take(150) // Limit filename length
        .collect::<String>()
}

fn is_binary_file(path: &Path) -> io::Result<bool> {
    // Read the first 8KB of the file
    let mut buffer = [0; 8192];
    
    match File::open(path) {
        Ok(mut file) => {
            let bytes_read = match file.read(&mut buffer) {
                Ok(bytes) => bytes,
                Err(_) => return Ok(true), // If we can't read, assume binary
            };
            
            // If file is empty, it's not binary
            if bytes_read == 0 {
                return Ok(false);
            }
            
            // Check for null bytes or other binary indicators
            for &byte in &buffer[..bytes_read] {
                if byte == 0 {
                    return Ok(true);
                }
            }
        },
        Err(_) => return Ok(true), // If we can't open, assume binary
    }
    
    // Check file extension for common binary formats
    let extension = path.extension().and_then(|ext| ext.to_str()).unwrap_or("");
    let binary_extensions = [
        "png", "jpg", "jpeg", "gif", "bmp", "tiff",
        "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
        "zip", "tar", "gz", "rar", "7z",
        "exe", "dll", "so", "dylib", "bin",
        "mp3", "mp4", "wav", "avi", "mov",
    ];
    
    if binary_extensions.contains(&extension.to_lowercase().as_str()) {
        return Ok(true);
    }
    
    Ok(false)
}

fn is_too_large(path: &Path) -> io::Result<bool> {
    // Get current metadata
    match fs::metadata(path) {
        Ok(metadata) => {
            // Default threshold: 1MB (will be overridden by max_size parameter)
            Ok(metadata.len() > 1_000_000)
        },
        Err(e) => {
            eprintln!("Couldn't get metadata for {}: {}", path.display(), e);
            // If we can't determine size, assume it's not too large
            Ok(false)
        }
    }
}