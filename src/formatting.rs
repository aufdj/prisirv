use std::{
    path::{Path, PathBuf, Component},
    fs::create_dir_all,
    ffi::OsStr,
};
use crate::{ 
    config::{Config, Mode},
    filedata::FileData,
};


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
        cfg.ex_arch.path.with_extension("")
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
/// i.e. \foo + \bar\baz\qux.txt -> \foo\baz\qux.txt
///
/// If the parent directory of the output path doesn't exist, it and other 
/// required directories are created.
pub fn fmt_file_out_extract(dir_out: &FileData, file_in: &FileData) -> FileData {
    let first = file_in.path.components().next();

    let mut file_cmpnts = file_in.path.components();

    // If path contains prefix, advance iterator twice to make the path
    // non-absolute so join() will actually join rather than replace.
    // If path contains root, advance iterator once to make the path
    // non-absolute so join() will actually join rather than replace. 
    match first {
        Some(Component::Prefix(_)) => {
            file_cmpnts.next().unwrap();
            file_cmpnts.next().unwrap();
        },
        Some(Component::RootDir) => {
            file_cmpnts.next().unwrap();
        },
        _ => {}
    }

    let mut d = dir_out.path.into_iter();
    
    let n = file_cmpnts
        .as_path()
        .into_iter()
        .filter(|&f| !d.any(|m| f == m))
        .collect::<Vec<&OsStr>>();

    let mut end = PathBuf::new();
    for s in n.iter() {
        end.push(s);
    }

    let path = dir_out.path.join(end);

    if let Some(parent) = path.parent() {
        if !parent.exists() {
            create_dir_all(parent).unwrap();
        }
    }
    
    let mut file_out = FileData::new(path);
    file_out.seg_beg = file_in.seg_beg;
    file_out
}

