mod buffered_io;
mod logistic;
mod statemap;
mod apm;
mod mixer;
mod match_model;
mod hash_table;
mod metadata;
mod predictor;
mod encoder;
mod decoder;

use std::{
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
    cmp::{min, Ordering},
    time::Instant,
    env,  
};
use crate::{
    buffered_io::{
    BufferedRead, BufferedWrite, BufferState,
    new_input_file, new_output_file, new_dir
    },
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata
};

const MEM: usize = 1 << 23;


// Get file name or path without extension
fn file_name_no_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
    .split(".").next().unwrap()
}
fn file_path_no_ext(path: &Path) -> &str {
    path.to_str().unwrap()
    .split(".").next().unwrap()
}
// Get file name or path with extension 
fn file_name_ext(path: &Path) -> &str {
    path.file_name().unwrap()
    .to_str().unwrap()
}
fn file_path_ext(path: &Path) -> String {
    path.to_str().unwrap().to_string()
}
fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}


// Non-solid archiving --------------------------------------------------------------------------------------------------------------------
fn compress_file(file_in_path: &Path, dir_out: &String) -> u64 {
    let mut mta: Metadata = Metadata::new();
    
    // Create output file path from current output directory
    // and input file name without extension
    // i.e. foo/bar.txt -> foo/bar.pri
    let file_out_path = 
        PathBuf::from(
            &format!("{}\\{}.pri",  
            dir_out, file_name_no_ext(file_in_path))
        );  

    // Create input file with buffer = block size
    let mut file_in = new_input_file(mta.bl_sz, file_in_path);
    let mut enc = Encoder::new(new_output_file(4096, &file_out_path));
    
    // Set metadata extension field
    mta.set_ext(file_in_path);

    // Compress
    loop {
        if file_in.fill_buffer() == BufferState::Empty { break; }
        mta.f_bl_sz = file_in.buffer().len();
        enc.compress_block(&file_in.buffer());
        mta.bl_c += 1;
    } 
    
    enc.flush();
    enc.write_header(&mta);
    file_len(&file_out_path)
}
fn decompress_file(file_in_path: &Path, dir_out: &String) -> u64 {
    let mut dec = Decoder::new(new_input_file(4096, file_in_path));
    let mta: Metadata = dec.read_header();

    // Create output file path from current output directory,
    // input file name without extension, and file's original
    // extension (stored in header)
    // i.e. foo/bar.pri -> foo/bar.txt
    let file_out_path =
        PathBuf::from(
            &format!("{}\\{}.{}",
            dir_out,
            file_name_no_ext(file_in_path),
            mta.get_ext())
        );
    let mut file_out = new_output_file(4096, &file_out_path);
    
    // Call after reading header
    dec.init_x();

    // Decompress
    for _ in 0..(mta.bl_c - 1) {
        let block = dec.decompress_block(mta.bl_sz);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }
    }
    let block = dec.decompress_block(mta.f_bl_sz);
    for byte in block.iter() {
        file_out.write_byte(*byte);
    }

    file_out.flush_buffer();
    file_len(&file_out_path)
}
fn compress_dir(dir_in: &Path, dir_out: &mut String) {
    // Create new nested directory from current output 
    // directory and input directory name 
    let mut dir_out = 
        format!("{}\\{}", 
        dir_out, file_name_ext(dir_in));
    new_dir(&dir_out);

    // Sort files and directories
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    // Compress files first, then directories
    for file_in in files.iter() {
        let time = Instant::now();
        println!("Compressing {}", file_in.display());
        let file_in_size  = file_len(&file_in); 
        let file_out_size = compress_file(&file_in, &dir_out);
        println!("{} bytes -> {} bytes in {:.2?}\n", 
            file_in_size, file_out_size, time.elapsed());  
    }
    for dir_in in dirs.iter() {
        compress_dir(dir_in, &mut dir_out); 
    }
}
fn decompress_dir(dir_in: &Path, dir_out: &mut String, root: bool) {
    // Create new nested directory from current output 
    // directory and input directory name; if current output
    // directory is root, replace rather than nest
    let mut dir_out = 
        if root { dir_out.to_string() }
        else { 
            format!("{}\\{}", 
            dir_out, file_name_ext(dir_in)) 
        };
    if !Path::new(&dir_out).is_dir() {
        new_dir(&dir_out);
    }
    
    // Sort files and directories
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    // Decompress files first, then directories
    for file_in in files.iter() {
        let time = Instant::now();
        println!("Decompressing {}", file_in.display());
        let file_in_size  = file_len(&file_in);
        let file_out_size = decompress_file(&file_in, &dir_out);
        println!("{} bytes -> {} bytes in {:.2?}\n",
            file_in_size, file_out_size, time.elapsed());
    }
    for dir_in in dirs.iter() {
        decompress_dir(&dir_in, &mut dir_out, false); 
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Solid archiving ------------------------------------------------------------------------------------------------------------------------
enum Sort {   // Sort By:
    None,     // No sorting
    Ext,      // Extension
    PrtDir,   // Parent Directory
    Created,  // Creation Time
    Accessed, // Last Access Time
    Modified, // Last Modification Time
}
fn sort_ext(f1: &String, f2: &String) -> Ordering {
        (Path::new(f1).extension().unwrap())
    .cmp(Path::new(f2).extension().unwrap())
}
fn sort_prtdir(f1: &String, f2: &String) -> Ordering {
        (Path::new(f1).parent().unwrap())
    .cmp(Path::new(f2).parent().unwrap())
}
fn sort_created(f1: &String, f2: &String) -> Ordering {
         (Path::new(f1).metadata().unwrap().created().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().created().unwrap())
}
fn sort_accessed(f1: &String, f2: &String) -> Ordering {
         (Path::new(f1).metadata().unwrap().accessed().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().accessed().unwrap())
}
fn sort_modified(f1: &String, f2: &String) -> Ordering {
         (Path::new(f1).metadata().unwrap().modified().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().modified().unwrap())
}
fn collect_files(dir_in: &Path, mta: &mut Metadata) {
    let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
        dir_in.read_dir().unwrap()
        .map(|d| d.unwrap().path())
        .partition(|f| f.is_file());

    for file in files.iter() {
        mta.files.push(
            (file_path_ext(file), 0, 0)
        );
    }
    if !dirs.is_empty() {
        for dir in dirs.iter() {
            collect_files(dir, mta);
        }
    }
}
fn compress_file_solid(enc: &mut Encoder, mta: &mut Metadata, curr_file: usize) {
    // Create input file with buffer = block size
    let mut file_in = new_input_file(mta.bl_sz, Path::new(&mta.files[curr_file].0));

    // Compress
    loop {
        if file_in.fill_buffer() == BufferState::Empty { break; }
        mta.files[curr_file].2 = file_in.buffer().len();
        enc.compress_block(&file_in.buffer());
        mta.files[curr_file].1 += 1;
    }
    println!("Total archive size: {}\n", 
    enc.file_out.stream_position().unwrap());
}
fn decompress_file_solid(dec: &mut Decoder, mta: &mut Metadata, dir_out: &String, curr_file: usize) {
    let file_out_name =
        format!("{}\\{}",
            dir_out,
            file_name_ext(
                Path::new(&mta.files[curr_file].0)
            ),
        );
    let mut file_out = new_output_file(4096, Path::new(&file_out_name));

    // Decompress
    for _ in 0..((mta.files[curr_file].1) - 1) {
        let block = dec.decompress_block(mta.bl_sz);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }
    }
    let block = dec.decompress_block(mta.files[curr_file].2);
    for byte in block.iter() {
        file_out.write_byte(*byte);
    }
    file_out.flush_buffer();
}
// ----------------------------------------------------------------------------------------------------------------------------------------

#[derive(PartialEq, Eq)]
enum Format {
    Archive,
    Extract,
    ArchiveSolid,
    ExtractSolid,
}

fn format_dir_out(fmt: Format, user_out: &String, arg: &PathBuf) -> String {
    let mut dir_out = String::new();
    if user_out.is_empty() {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_pri", file_path_no_ext(&arg)),
            Format::Extract =>      format!("{}_d",   file_path_no_ext(&arg)),
            Format::ArchiveSolid => format!("{}.pri", file_path_no_ext(&arg)),
            Format::ExtractSolid => format!("{}_d",   file_path_no_ext(&arg)),
        }    
    }
    else {
        // An -out option containing \'s will be treated as an absolute path.
        // An -out option with no \'s creates a new archive inside the same directory as the first input.
        // i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive \baz\arch,
        // while option '-out arch' creates archive \foo\arch.
        if user_out.contains("\\") {
            dir_out = match fmt {
                Format::Archive =>      format!("{}_pri", user_out),
                Format::Extract =>      format!("{}_d",   user_out),
                Format::ArchiveSolid => format!("{}.pri", user_out),
                Format::ExtractSolid => format!("{}_d",   user_out),
            }    
        }
        else {
            let s: Vec<String> = 
                file_path_ext(&arg)
                .split("\\").skip(1)
                .map(|s| s.to_string())
                .collect();
            for cmpnt in s.iter().take(s.len()-1) {
                dir_out.push_str(format!("\\{}", cmpnt).as_str());
            }
            dir_out = match fmt {
                Format::Archive =>      format!("{}\\{}_pri", dir_out, user_out),
                Format::Extract =>      format!("{}\\{}_d",   dir_out, user_out),
                Format::ArchiveSolid => format!("{}\\{}.pri", dir_out, user_out),
                Format::ExtractSolid => format!("{}\\{}_d",   dir_out, user_out),
            }    
        }   
    }
    dir_out
}

fn main() {
    // Get arguments, skipping over program name
    let mut args = env::args().skip(1).peekable();

    // Print program info
    if args.peek() == None {
        println!();
        println!("      ______   ______     ________  ______    ________  ______    __   __     
     /_____/\\ /_____/\\   /_______/\\/_____/\\  /_______/\\/_____/\\  /_/\\ /_/\\    
     \\:::_ \\ \\\\:::_ \\ \\  \\__.::._\\/\\::::_\\/_ \\__.::._\\/\\:::_ \\ \\ \\:\\ \\\\ \\ \\   
      \\:(_) \\ \\\\:(_) ) )_   \\::\\ \\  \\:\\/___/\\   \\::\\ \\  \\:(_) ) )_\\:\\ \\\\ \\ \\  
       \\: ___\\/ \\: __ `\\ \\  _\\::\\ \\__\\_::._\\:\\  _\\::\\ \\__\\: __ `\\ \\\\:\\_/.:\\ \\ 
        \\ \\ \\    \\ \\ `\\ \\ \\/__\\::\\__/\\ /____\\:\\/__\\::\\__/\\\\ \\ `\\ \\ \\\\ ..::/ / 
         \\_\\/     \\_\\/ \\_\\/\\________\\/ \\_____\\/\\________\\/ \\_\\/ \\_\\/ \\___/_(  
                                                                                 ");
        println!();
        println!("Prisirv is a context mixing archiver based on lpaq1");
        println!("Source code available at https://github.com/aufdj/prisirv");
        println!();
        println!("USAGE: PROG_NAME [c|d] [-out [path]] [-sld] [-sort [..]] [files|dirs]");
        println!();
        println!("OPTIONS:");
        println!("   c      Compress");
        println!("   d      Decompress");
        println!("  -out    Specify output path");
        println!("  -sld    Create solid archive");
        println!("  -sort   Sort files (solid archives only)");
        println!();
        println!("      Sorting Methods:");
        println!("          -sort ext      Sort by extension");
        println!("          -sort prtdir   Sort by parent directory");
        println!("          -sort crtd     Sort by creation time");
        println!("          -sort accd     Sort by last access time");
        println!("          -sort mod      Sort by last modification time");
        println!();
        println!("EXAMPLE:");
        println!("  Compress file [\\foo\\bar.txt] and directory [baz] into solid archive [\\foo\\arch], \n  sorting files by creation time:");
        println!();
        println!("      prisirv c -out arch -sld -sort crtd \\foo\\bar.txt \\baz");
        println!();
        println!("  Decompress the archive:");
        println!();
        println!("      prisirv d -sld \\foo.pri");
        std::process::exit(0);
    }

    // Get mode
    let mode = args.next().unwrap();

    // Get user specified output path
    let mut user_out = String::new();
    if args.peek().unwrap() == "-out" {
        args.next();
        user_out = args.next().unwrap();
    }

    // Determine if solid or non-solid archive
    let mut solid = false;
    if args.peek().unwrap() == "-sld" { 
        solid = true;
        args.next();
    }

    // Select sorting option
    let mut sort = Sort::None;
    if args.peek().unwrap() == "-sort" { 
        args.next();
        sort = match args.next().unwrap().as_str() {
            "ext"    => Sort::Ext,
            "prtdir" => Sort::PrtDir,
            "crtd"   => Sort::Created,
            "accd"   => Sort::Accessed,
            "mod"    => Sort::Modified,
            _ => { 
                println!("Not a valid sort criteria.");
                std::process::exit(1);
            }
        }
    }

    let mut dir_out = String::new();

    if solid {
        let mut mta: Metadata = Metadata::new();
        match mode.as_str() {
            "c" => {
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() || arg.is_dir() {
                    dir_out = format_dir_out(Format::ArchiveSolid, &user_out, &arg);
                }
                else {
                    println!("No files or directories found.");
                    std::process::exit(1);
                }

                // Sort files and directories
                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
                    args.map(|f| PathBuf::from(f))
                    .partition(|f| f.is_file());

                // Add file paths and lengths to metadata
                for file in files.iter() {
                    mta.files.push(
                        (file_path_ext(file), 0, 0)
                    );
                }
                
                // Walk through directories and gather rest of files
                if !dirs.is_empty() {
                    for dir in dirs.iter() {
                        collect_files(dir, &mut mta);
                    }
                }

                // Sort files to potentially improve compression of solid archives
                match sort {
                    Sort::None     => {},
                    Sort::Ext      => mta.files.sort_by(|f1, f2| sort_ext(&f1.0, &f2.0)),
                    Sort::PrtDir   => mta.files.sort_by(|f1, f2| sort_prtdir(&f1.0, &f2.0)),
                    Sort::Created  => mta.files.sort_by(|f1, f2| sort_created(&f1.0, &f2.0)),
                    Sort::Accessed => mta.files.sort_by(|f1, f2| sort_accessed(&f1.0, &f2.0)),
                    Sort::Modified => mta.files.sort_by(|f1, f2| sort_modified(&f1.0, &f2.0)),
                }

                let mut enc = Encoder::new(new_output_file(4096, Path::new(&dir_out)));
                
                for curr_file in 0..mta.files.len() {
                    println!("Compressing {}", mta.files[curr_file].0);
                    compress_file_solid(&mut enc, &mut mta, curr_file);
                }
                enc.flush();
                
                // Get index to end of file metadata
                mta.f_ptr =
                    enc.file_out.stream_position()
                    .unwrap() as usize;

                // Output number of files
                enc.file_out.write_usize(mta.files.len());

                for file in mta.files.iter() {
                    // Get path as byte slice, truncated if longer than 255 bytes
                    let path = &file.0.as_bytes()[..min(file.0.len(), 255)];

                    // Output length of file path (for parsing)
                    enc.file_out.write_byte(path.len() as u8);

                    // Output path
                    for byte in path.iter() {
                        enc.file_out.write_byte(*byte);
                    }

                    // Output block count and final block size
                    enc.file_out.write_usize(file.1);
                    enc.file_out.write_usize(file.2);
                }

                // Go back to beginning of file and write header
                enc.file_out.rewind().unwrap();
                enc.write_header(&mta);
            }
            "d" => {
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() {
                    dir_out = format_dir_out(Format::ExtractSolid, &user_out, &arg);
                    new_dir(&dir_out);
                }
                else {
                    println!("No solid archives found.");
                    std::process::exit(1);
                }

                let mut dec = Decoder::new(new_input_file(4096, &arg));
                mta = dec.read_header();

                // Seek to end of file metadata
                dec.file_in.seek(SeekFrom::Start(mta.f_ptr as u64)).unwrap();

                // Parse files and lengths
                let mut path: Vec<u8> = Vec::new();

                // Get number of files
                let num_files = dec.file_in.read_usize();

                for _ in 0..num_files {
                    // Get length of next path
                    let len = dec.file_in.read_byte();

                    // Get path, block count, final block size
                    for _ in 0..len {
                        path.push(dec.file_in.read_byte());
                    }
                    mta.files.push(
                        (path.iter().cloned()
                            .map(|b| b as char)
                            .collect::<String>(),
                         dec.file_in.read_usize(),
                         dec.file_in.read_usize())
                    );
                    path.clear();
                }

                // Seek back to beginning of compressed data
                dec.file_in.seek(SeekFrom::Start(40)).unwrap();

                dec.init_x();
                
                for curr_file in 0..mta.files.len() {
                    println!("Decompressing {}", mta.files[curr_file].0);
                    decompress_file_solid(&mut dec, &mut mta, &dir_out, curr_file);
                }
            }
            _ => {
                println!("To Compress: c input");
                println!("To Decompress: d input");
            }
        }
    }
    else {
        match mode.as_str() {
            "c" => {
                // Create archive with same name as first file/dir
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() || arg.is_dir() {
                    dir_out = format_dir_out(Format::Archive, &user_out, &arg);
                    new_dir(&dir_out);
                }
                else {
                    println!("No files or directories found.");
                    std::process::exit(1);
                }

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    args.map(|f| PathBuf::from(f))
                    .partition(|f| f.is_file());
            
                for file_in in files.iter() {
                    let time = Instant::now();
                    println!("Compressing {}", file_in.display());
                    let file_in_size  = file_len(&file_in); 
                    let file_out_size = compress_file(&file_in, &dir_out);
                    println!("{} bytes -> {} bytes in {:.2?}\n", 
                    file_in_size, file_out_size, time.elapsed());  
                }
                for dir_in in dirs.iter() {
                    compress_dir(dir_in, &mut dir_out);      
                }
            }
            "d" => {
                // Create archive with same name as first file/dir
                let arg = PathBuf::from(&args.peek().unwrap());
                if arg.is_file() || arg.is_dir() {
                    dir_out = format_dir_out(Format::Extract, &user_out, &arg);
                    new_dir(&dir_out);
                }
                else {
                    println!("No files or directories found.");
                    std::process::exit(1);
                }

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    args.map(|f| PathBuf::from(f))
                    .partition(|f| f.is_file());
            
                for file_in in files.iter() {
                    let time = Instant::now();
                    println!("Decompressing {}", file_in.display());
                    let file_in_size  = file_len(&file_in); 
                    let file_out_size = decompress_file(&file_in, &dir_out);
                    println!("{} bytes -> {} bytes in {:.2?}\n", 
                    file_in_size, file_out_size, time.elapsed());  
                }
                for dir_in in dirs.iter() {
                    decompress_dir(dir_in, &mut dir_out, true);      
                }
            }
            _ => {
                println!("To Compress: c input");
                println!("To Decompress: d input");
            }
        }
    }        
}





