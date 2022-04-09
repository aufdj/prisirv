use std::{
    path::{Path, PathBuf},
    fs::create_dir_all,
};
use crate::{
    Mode, 
    config::Config,
    metadata::FileData,
};


trait PathFmt {
    fn name_no_ext(&self) -> &str;
    fn path_no_ext(&self) -> &str;
    fn name_ext(&self) -> &str;
    fn path_ext(&self) -> String;
}

impl PathFmt for Path {
    /// Get file name without extension.
    fn name_no_ext(&self) -> &str {
        self.file_name().unwrap()
        .to_str().unwrap()
        .split('.').next().unwrap()
    }

    /// Get file path without extension.
    fn path_no_ext(&self) -> &str {
        self.to_str().unwrap()
        .split('.').next().unwrap()
    }

    /// Get file name with extension.
    fn name_ext(&self) -> &str {
        self.file_name().unwrap()
        .to_str().unwrap()
    }

    /// Get file path with extension.
    fn path_ext(&self) -> String {
        self.to_str().unwrap().to_string()
    }
}


/// Used for determining archive name.
#[derive(PartialEq, Eq)]
enum Format {
    Archive,
    Extract,
}


/// Creates a new archive or extracted archive path.
pub fn fmt_root_output_dir(cfg: &Config) -> FileData {
    match cfg.mode {
        Mode::Compress => {
            FileData::new(
                PathBuf::from(
                    fmt_dir_out(Format::Archive, &cfg.user_out, &cfg.inputs[0].path)
                )
            )
        }
        Mode::Decompress => {
            FileData::new(
                PathBuf::from(
                    fmt_dir_out(Format::Extract, &cfg.user_out, &cfg.inputs[0].path)
                )
            )
        }
    }
}

/// Format output directory given a format, an optional user specified output, 
/// and the first input file or directory.
///
/// An -out option containing \'s will be treated as an absolute path.
///
/// An -out option with no \'s creates a new archive inside the same directory 
/// as the first input.
///
/// i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive 
/// \baz\arch, while option '-out arch' creates archive \foo\arch.
fn fmt_dir_out(fmt: Format, user_out: &str, first_input: &Path) -> String {
    let mut dir_out = String::new();
    if user_out.is_empty() {
        dir_out = match fmt {
            Format::Archive => format!("{}.prsv", first_input.path_no_ext()),
            Format::Extract => format!("{}_d",    first_input.path_no_ext()),
        }    
    }
    else if user_out.contains('\\') {
        dir_out = match fmt {
            Format::Archive => format!("{}.prsv", user_out),
            Format::Extract => format!("{}_d",    user_out),
        }    
    }
    else {
        // Replace final path component with user option
        let s: Vec<String> = 
            first_input.path_ext()
            .split('\\').skip(1)
            .map(|s| s.to_string())
            .collect();
        for cmpnt in s.iter().take(s.len()-1) {
            dir_out.push_str(format!("\\{}", cmpnt).as_str());
        }
        dir_out = match fmt {
            Format::Archive => format!("{}\\{}.prsv", dir_out, user_out),
            Format::Extract => format!("{}\\{}_d",    dir_out, user_out),
        }    
    }   
    dir_out
}

/// Format output file in extracted solid archive
///
/// Reconstruct original directory structure based on output directory and 
/// absolute path of the file being compressed. This is done by chaining 
/// together the output directory path and the input file path, excluding 
/// the top level of the input path.
/// i.e. \foo_d + \foo\bar\baz.txt -> \foo_d\bar\baz.txt
///
/// If the parent directory of the output path doesn't exist, it and other 
/// required directories are created.
pub fn fmt_file_out_s_extract(dir_out: &str, file_in_path: &Path) -> PathBuf {
    let dir_out_path = Path::new(dir_out);

    let path = 
    if dir_out_path.is_absolute() {
        PathBuf::from(
            dir_out_path.iter()
            .skip(1)
            .chain(
                file_in_path.iter()
                .filter(|c| c.to_str().unwrap() != "C:")
                .filter(|c| c.to_str().unwrap() != "\\")
            )
            .map(|s| format!("\\{}", s.to_str().unwrap()))
            .skip(1)
            .collect::<String>()
        )
    }
    else if dir_out_path.starts_with("\\") {
        PathBuf::from(
            dir_out_path.iter()
            .chain(
                file_in_path.iter()
                .filter(|c| c.to_str().unwrap() != "C:")
                .filter(|c| c.to_str().unwrap() != "\\")
            )
            .map(|s| format!("\\{}", s.to_str().unwrap()))
            .skip(1)
            .collect::<String>()
        )
    }
    else {
        PathBuf::from(
            dir_out_path.iter()
            .chain(
                file_in_path.iter()
                .filter(|c| c.to_str().unwrap() != "C:")
                .filter(|c| c.to_str().unwrap() != "\\")
            )
            .map(|s| format!("\\{}", s.to_str().unwrap()))
            .collect::<String>().strip_prefix('\\').unwrap()
        )
    };

    let parent = path.parent().unwrap();
    if !parent.exists() {
        create_dir_all(parent).unwrap();
    }
    
    path
}

