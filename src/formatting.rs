use std::{
    path::{Path, PathBuf},
    fs::create_dir_all,
};
use crate::{ 
    config::{Config, Mode},
    filedata::FileData,
};


pub trait PathFmt {
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


/// Format output directory given an optional user specified output, and the 
/// first input file or directory.
///
/// An -output-path option containing \'s will be treated as an absolute path.
///
/// An -output-path option with no \'s creates a new archive inside the same 
/// directory as the first input.
///
/// i.e. Compressing \foo\bar.txt with option '-output-path \baz\arch' creates 
/// archive \baz\arch, while option '-output-path arch' creates archive \foo\arch.
pub fn fmt_root_output(cfg: &Config) -> FileData {
    let mut out = 
    if cfg.user_out.is_empty() {
        cfg.inputs[0].path.path_no_ext().to_string() 
    }
    else if cfg.user_out.contains('\\') {
        cfg.user_out.to_string()
    }
    else {
        let mut dir_out = String::new();
        // Replace final path component with user option
        let s: Vec<String> = 
            cfg.inputs[0].path.path_ext()
            .split('\\').skip(1)
            .map(|s| s.to_string())
            .collect();
        for cmpnt in s.iter().take(s.len()-1) {
            dir_out.push_str(format!("\\{}", cmpnt).as_str());
        }
        format!("{}\\{}", dir_out, cfg.user_out)
    };

    match cfg.mode {
        Mode::CreateArchive => {
            out.push_str(".prsv");
        }
        Mode::ExtractArchive => {
            out.push_str("_d");
        }
        Mode::AddFiles => {
            out.push_str("_add.prsv");
        }
        Mode::ExtractFiles => {
            out.push_str("_d");
        }
    }
    FileData::new(PathBuf::from(out))
}

/// Format output file in extracted archive
///
/// Reconstruct original directory structure based on output directory and 
/// absolute path of the file being compressed. This is done by chaining 
/// together the output directory path and the input file path, excluding 
/// the top level of the input path.
/// i.e. \foo_d + \foo\bar\baz.txt -> \foo_d\bar\baz.txt
///
/// If the parent directory of the output path doesn't exist, it and other 
/// required directories are created.
pub fn fmt_file_out_extract(dir_out: &str, file_in_path: &Path) -> FileData {
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
    
    FileData::new(path)
}

