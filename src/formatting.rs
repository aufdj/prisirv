use std::{
    path::{Path, PathBuf},
    fs::create_dir_all,
};
use crate::{Mode, parse_args::Config};

// Get file name or path without extension
fn file_name_no_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
    .split('.').next().unwrap()
}
fn file_path_no_ext(path: &Path) -> &str {
    path.to_str().unwrap()
    .split('.').next().unwrap()
}
// Get file name or path with extension 
fn file_name_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
}
fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}


#[derive(PartialEq, Eq)]
enum Format {
    Archive,
    Extract,
    ArchiveSolid,
    ExtractSolid,
}

// Formats the root directory path of the archive, based on the specified cli args.
pub fn format_root_output_dir(cfg: &Config, first_input_path: &Path) -> String {
    match cfg.mode {
        Mode::Compress => {
            if cfg.solid { format_dir_out(Format::ArchiveSolid, &cfg.user_out, first_input_path) }
            else         { format_dir_out(Format::Archive,      &cfg.user_out, first_input_path) }
        }
        Mode::Decompress => {
            if cfg.solid { format_dir_out(Format::ExtractSolid, &cfg.user_out, first_input_path) }
            else         { format_dir_out(Format::Extract,      &cfg.user_out, first_input_path) }
        }
    }
}

// An -out option containing \'s will be treated as an absolute path.
// An -out option with no \'s creates a new archive inside the same directory as the first input.
// i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive \baz\arch,
// while option '-out arch' creates archive \foo\arch.
fn format_dir_out(fmt: Format, user_out: &str, arg: &Path) -> String {
    let mut dir_out = String::new();
    if user_out.is_empty() {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_prsv", file_path_no_ext(arg)),
            Format::Extract =>      format!("{}_d",    file_path_no_ext(arg)),
            Format::ArchiveSolid => format!("{}.prsv", file_path_no_ext(arg)),
            Format::ExtractSolid => format!("{}_d",    file_path_no_ext(arg)),
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
            file_path_ext(arg)
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


pub fn format_file_out_path_ns_archive(dir_out: &str, file_in_path: &Path, clbr: bool) -> PathBuf {
    let mut dup = 1;
    // Create output file path from current output directory
    // and input file name without extension
    // i.e. foo/bar.txt -> foo/bar.prsv
    let mut file_out_path = 
        PathBuf::from(
            &format!("{}\\{}.prsv",  
            dir_out, file_name_no_ext(file_in_path))
        ); 
    
    // Modify file path if it already exists due to extension change
    // i.e foo/bar.txt -> foo/bar.prsv
    //     foo/bar.bin -> foo/bar.prsv -> foo/bar(1).prsv
    while file_out_path.exists() && !clbr {
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

pub fn format_nested_dir_path_ns_archive(dir_out: &str, dir_in: &Path) -> String {
    // Create new nested directory from current output 
    // directory and input directory name 
    format!("{}\\{}", 
    dir_out, file_name_ext(dir_in))
}

pub fn format_file_out_path_ns_extract(ext: &str, dir_out: &str, file_in_path: &Path) -> PathBuf {
    // Create output file path from current output directory,
    // input file name without extension, and file's original
    // extension (stored in header)
    // i.e. foo/bar.prsv -> foo/bar.txt
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

pub fn format_nested_dir_path_ns_extract(dir_out: &str, dir_in: &Path, root: bool) -> String {
    // Create new nested directory from current output 
    // directory and input directory name. If current output
    // directory is root, replace rather than nest.
    if root { dir_out.to_string() }
    else { 
        format!("{}\\{}", 
        dir_out, file_name_ext(dir_in)) 
    }
}

pub fn format_file_out_path_s_extract(dir_out: &str, file_in_path: &Path) -> PathBuf {
    // Reconstruct original directory structure based on output directory  
    // and absolute path of the file being compressed. 
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
