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
    fn path_ext(&self) -> &str;
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
    /// Get file path with extension.
    fn path_ext(&self) -> &str {
        self.to_str().unwrap()
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
        PathBuf::from(cfg.ex_arch.path.path_no_ext())
    }
    else if cfg.user_out.contains('\\') {
        PathBuf::from(&cfg.user_out)
    }
    else {
        let mut cmpnts = cfg.ex_arch.path.components();
        cmpnts.next_back().unwrap();

        cmpnts.as_path().join(Path::new(&cfg.user_out))
    };

    if cfg.mode == Mode::CreateArchive {
        out.set_extension("prsv");
    }
    FileData::new(out)
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
pub fn fmt_file_out_extract(dir_out: &str, file_in: &FileData) -> FileData {
    let dir_out_path = Path::new(dir_out);

    let path = 
    if dir_out_path.is_absolute() {
        PathBuf::from(
            dir_out_path.iter()
            .skip(1)
            .chain(
                file_in.path.iter()
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
                file_in.path.iter()
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
                file_in.path.iter()
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
    
    let mut file_out = FileData::new(path);
    file_out.seg_beg = file_in.seg_beg;
    file_out
}

