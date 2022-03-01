use std::{
    path::{Path, PathBuf},
    fs::create_dir_all,
};
use crate::{Mode, Arch};

/// Get file name without extension.
fn file_name_no_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
    .split('.').next().unwrap()
}
/// Get file path without extension.
fn file_path_no_ext(path: &Path) -> &str {
    path.to_str().unwrap()
    .split('.').next().unwrap()
}
/// Get file name with extension.
fn file_name_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
}
/// Get file path with extension.
fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}


/// Used for determining archive name.
#[derive(PartialEq, Eq)]
enum Format {
    Archive,
    Extract,
    ArchiveSolid,
    ExtractSolid,
}


/// Creates a new archive or extracted archive path. This is 
/// a directory, except in the case of a solid archive, where
/// it is a file. The user can specify an output path, otherwise
/// a default will be chosen.
pub fn fmt_root_output_dir(arch: Arch, mode: Mode, user_out: &str, first_input: &Path) -> String {
    match (arch, mode) {
        (Arch::Solid, Mode::Compress) => {
            fmt_dir_out(Format::ArchiveSolid, user_out, first_input)
        }
        (Arch::Solid, Mode::Decompress) => {
            fmt_dir_out(Format::ExtractSolid, user_out, first_input)   
        }
        (Arch::NonSolid, Mode::Compress) => {
            fmt_dir_out(Format::Archive,      user_out, first_input)
        }
        (Arch::NonSolid, Mode::Decompress) => {
            fmt_dir_out(Format::Extract,      user_out, first_input)
        }
    }
}

/// Format output directory given a format, an optional user  
/// specified output, and the first input file or directory
///
/// An -out option containing \'s will be treated as an absolute path.
///
/// An -out option with no \'s creates a new archive inside the same 
/// directory as the first input.
///
/// i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive \baz\arch,
/// while option '-out arch' creates archive \foo\arch.
fn fmt_dir_out(fmt: Format, user_out: &str, first_input: &Path) -> String {
    let mut dir_out = String::new();
    if user_out.is_empty() {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_prsv", file_path_no_ext(first_input)),
            Format::Extract =>      format!("{}_d",    file_path_no_ext(first_input)),
            Format::ArchiveSolid => format!("{}.prsv", file_path_no_ext(first_input)),
            Format::ExtractSolid => format!("{}_d",    file_path_no_ext(first_input)),
        }    
    }
    else if user_out.contains('\\') {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_prsv", user_out),
            Format::Extract =>      format!("{}_d",    user_out),
            Format::ArchiveSolid => format!("{}.prsv", user_out),
            Format::ExtractSolid => format!("{}_d",    user_out),
        }    
    }
    else {
        // Replace final path component with user option
        let s: Vec<String> = 
            file_path_ext(first_input)
            .split('\\').skip(1)
            .map(|s| s.to_string())
            .collect();
        for cmpnt in s.iter().take(s.len()-1) {
            dir_out.push_str(format!("\\{}", cmpnt).as_str());
        }
        dir_out = match fmt {
            Format::Archive =>      format!("{}\\{}_prsv", dir_out, user_out),
            Format::Extract =>      format!("{}\\{}_d",    dir_out, user_out),
            Format::ArchiveSolid => format!("{}\\{}.prsv", dir_out, user_out),
            Format::ExtractSolid => format!("{}\\{}_d",    dir_out, user_out),
        }    
    }   
    dir_out
}

/// Format new output file in non-solid archive
///
/// Since each compressed file is given the .prsv extension, two files
/// with different extensions but identical names could overwrite
/// each other. To avoid this, a different file name is found for a duplicate file.
/// i.e. foo/bar.txt -> foo/bar.prsv
///      foo/bar.bin -> foo/bar.prsv -> foo/bar(1).prsv
pub fn fmt_file_out_ns_archive(dir_out: &str, file_in_path: &Path, clbr: bool, files: &[PathBuf]) -> PathBuf {
    // Variable for keeping track of number of duplicates.
    let mut dup = 1;

    // Create output file path from current output directory
    // and input file name without extension
    // i.e. foo/bar.txt -> foo/bar.prsv
    let mut file_out_path = 
        PathBuf::from(
            &format!("{}\\{}.prsv",  
            dir_out, file_name_no_ext(file_in_path))
        ); 
    
    // If file_out_path already exists, find different path.
    // If clobbering is enabled, this code won't run and an 
    // existing file will be overwitten. This is correct  
    // behavior if the existing file is an old one and not 
    // part of the new archive, but if there are duplicate 
    // files added to the same archive, the first file would 
    // be overwritten anyway. To avoid this, each newly 
    // compressed file is added to a Vec<PathBUf> files,   
    // and if a duplicate file found is in this list, ignore 
    // the clobbering option.
    while file_out_path.exists() && !clbr || files.contains(&file_out_path) {
        file_out_path = 
        if dup == 1 {
            PathBuf::from(
                &format!("{}({}).prsv", 
                file_path_no_ext(&file_out_path), dup)
            )
        }
        else {
            let file_path = file_path_no_ext(&file_out_path);
            PathBuf::from(
                &format!("{}({}).prsv", 
                // Replace number rather than append
                &file_path[..file_path.len()-3], dup)
            )
        };
        dup += 1;
    }
    file_out_path
}

/// Create new nested directory from current output 
/// directory and input directory name. 
pub fn fmt_nested_dir_ns_archive(dir_out: &str, dir_in: &Path) -> String {
    format!("{}\\{}", 
    dir_out, file_name_ext(dir_in))
}

/// Format output file in extracted non-solid archive
///
/// Create output file path from current output directory, 
/// input file name without extension, and file's original
/// extension, stored in the header.
/// i.e. foo/bar.prsv -> foo/bar.txt
pub fn fmt_file_out_ns_extract(ext: &str, dir_out: &str, file_in_path: &Path) -> PathBuf {
    if ext.is_empty() { // Handle no extension
        PathBuf::from(
            &format!("{}\\{}",
            dir_out, file_name_no_ext(file_in_path))
        )
    }
    else {
        PathBuf::from(
            &format!("{}\\{}.{}",
            dir_out, file_name_no_ext(file_in_path), ext)
        )
    }
}

/// Format new nested directory in extracted non-solid archive
///
/// Create new nested directory from current output 
/// directory and input directory name. If current 
/// output directory is root, replace rather than nest.
pub fn fmt_nested_dir_ns_extract(dir_out: &str, dir_in: &Path, root: bool) -> String {
    if root { 
        dir_out.to_string() 
    }
    else { 
        format!("{}\\{}", 
        dir_out, file_name_ext(dir_in)) 
    }
}

/// Format output file in extracted solid archive
///
/// Reconstruct original directory structure based on output directory  
/// and absolute path of the file being compressed. This is done by 
/// chaining together the output directory path and the input file path,
/// excluding the top level of the input path.
/// i.e. \foo_d + \foo\bar\baz.txt -> \foo_d\bar\baz.txt
///
/// If the parent directory of the output path doesn't exist, 
/// it and other required directories are created.
pub fn fmt_file_out_s_extract(dir_out: &str, file_in_path: &Path) -> PathBuf { 
    let path = 
        PathBuf::from(
            Path::new(dir_out).iter()
            .filter(|p| p.to_str().unwrap() != "C:")
            .chain(file_in_path.iter().skip(2))
            .map(|s| format!("\\{}", s.to_str().unwrap()))
            .skip(1)
            .collect::<String>()
        );
    let parent = path.parent().unwrap();
    if !parent.exists() {
        create_dir_all(parent).unwrap();
    }
    path
}
