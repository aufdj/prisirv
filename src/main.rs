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
mod tables;


use std::{
    io::{Seek, SeekFrom},
    path::{Path, PathBuf},
    cmp::{min, Ordering},
    time::Instant,
    env,  
};
use crate::{
    encoder::Encoder,
    decoder::Decoder,
    metadata::Metadata,
    buffered_io::{
        BufferedRead, BufferedWrite, BufferState,
        new_input_file, new_output_file, new_dir
    },
};

const MEM: usize = 1 << 23;


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
fn file_len(path: &Path) -> u64 {
    path.metadata().unwrap().len()
}


#[derive(Debug)]
enum Sort {   // Sort By:
    None,     // No sorting
    Ext,      // Extension
    PrtDir,   // Parent Directory
    Created,  // Creation Time
    Accessed, // Last Access Time
    Modified, // Last Modification Time
}
fn sort_ext(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).extension().unwrap().to_ascii_lowercase())
    .cmp(&Path::new(f2).extension().unwrap().to_ascii_lowercase())
}
fn sort_prtdir(f1: &str, f2: &str) -> Ordering {
        (Path::new(f1).parent().unwrap())
    .cmp(Path::new(f2).parent().unwrap())
}
fn sort_created(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).metadata().unwrap().created().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().created().unwrap())
}
fn sort_accessed(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).metadata().unwrap().accessed().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().accessed().unwrap())
}
fn sort_modified(f1: &str, f2: &str) -> Ordering {
         (Path::new(f1).metadata().unwrap().modified().unwrap())
    .cmp(&Path::new(f2).metadata().unwrap().modified().unwrap())
}


#[derive(PartialEq, Eq)]
enum Format {
    Archive,
    Extract,
    ArchiveSolid,
    ExtractSolid,
}

// An -out option containing \'s will be treated as an absolute path.
// An -out option with no \'s creates a new archive inside the same directory as the first input.
// i.e. Compressing \foo\bar.txt with option '-out \baz\arch' creates archive \baz\arch,
// while option '-out arch' creates archive \foo\arch.
fn format_dir_out(fmt: Format, user_out: &str, arg: &Path) -> String {
    let mut dir_out = String::new();
    if user_out.is_empty() {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_pri", file_path_no_ext(arg)),
            Format::Extract =>      format!("{}_d",   file_path_no_ext(arg)),
            Format::ArchiveSolid => format!("{}.pri", file_path_no_ext(arg)),
            Format::ExtractSolid => format!("{}_d",   file_path_no_ext(arg)),
        }    
    }
    else if user_out.contains('\\') {
        dir_out = match fmt {
            Format::Archive =>      format!("{}_pri", user_out),
            Format::Extract =>      format!("{}_d",   user_out),
            Format::ArchiveSolid => format!("{}.pri", user_out),
            Format::ExtractSolid => format!("{}_d",   user_out),
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
            Format::Archive =>      format!("{}\\{}_pri", dir_out, user_out),
            Format::Extract =>      format!("{}\\{}_d",   dir_out, user_out),
            Format::ArchiveSolid => format!("{}\\{}.pri", dir_out, user_out),
            Format::ExtractSolid => format!("{}\\{}_d",   dir_out, user_out),
        }    
    }   
    dir_out
}


fn print_program_info() {
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
    println!("USAGE: PROG_NAME [c|d] [-out [path]] [-sld] [-sort [..]] [-i [files|dirs]] [-q]");
    println!();
    println!("OPTIONS:");
    println!("   c      Compress");
    println!("   d      Decompress");
    println!("  -out    Specify output path");
    println!("  -sld    Create solid archive");
    println!("  -sort   Sort files (solid archives only)");
    println!("  -i      Denotes list of input files/dirs");
    println!("  -q      Suppresses output other than errors");
    println!();
    println!("      Sorting Methods:");
    println!("          -sort ext      Sort by extension");
    println!("          -sort prtdir   Sort by parent directory");
    println!("          -sort crtd     Sort by creation time");
    println!("          -sort accd     Sort by last access time");
    println!("          -sort mod      Sort by last modification time");
    println!();
    println!("EXAMPLE:");
    println!("  Compress file [\\foo\\bar.txt] and directory [\\baz] into solid archive [\\foo\\arch], \n  sorting files by creation time:");
    println!();
    println!("      prisirv c -out arch -sld -sort crtd -i \\foo\\bar.txt \\baz");
    println!();
    println!("  Decompress the archive:");
    println!();
    println!("      prisirv d -sld -i \\foo\\arch.pri");
    std::process::exit(0);
}


struct Archiver {
    dup: u32,
    quiet: bool,
}
impl Archiver {
    fn new(quiet: bool) -> Archiver {
        Archiver {
            dup: 1,
            quiet,
        }
    }
    // Non-solid archiving --------------------------------------------------------------------------------------------------------------------
    fn compress_file(&mut self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut mta: Metadata = Metadata::new();

        // Create output file path from current output directory
        // and input file name without extension
        // i.e. foo/bar.txt -> foo/bar.pri
        let mut file_out_path = 
            PathBuf::from(
                &format!("{}\\{}.pri",  
                dir_out, file_name_no_ext(file_in_path))
            ); 

        // Modify file path if it already exists due to extension change
        // i.e foo/bar.txt -> foo/bar.pri
        //     foo/bar.bin -> foo/bar.pri -> foo/bar(1).pri
        if file_out_path.exists() {
            file_out_path = 
            PathBuf::from(
                &format!("{}({}).pri", 
                file_path_no_ext(&file_out_path), self.dup)
            );
            self.dup += 1;
        }

        // Create input file with buffer = block size
        let mut file_in = new_input_file(mta.bl_sz, file_in_path);
        let mut enc = Encoder::new(new_output_file(4096, &file_out_path));

        // Set metadata extension field
        mta.set_ext(file_in_path);

        // Compress
        loop {
            if file_in.fill_buffer() == BufferState::Empty { break; }
            mta.f_bl_sz = file_in.buffer().len();
            enc.compress_block(file_in.buffer());
            mta.bl_c += 1;
        }

        enc.flush();
        enc.write_header(&mta);
        file_len(&file_out_path)
    }
    fn decompress_file(&self, file_in_path: &Path, dir_out: &str) -> u64 {
        let mut dec = Decoder::new(new_input_file(4096, file_in_path));
        let mta: Metadata = dec.read_header();

        // Create output file path from current output directory,
        // input file name without extension, and file's original
        // extension (stored in header)
        // i.e. foo/bar.pri -> foo/bar.txt
        let file_out_path = 
            // No extension
            if mta.get_ext().is_empty() {
                PathBuf::from(
                    &format!("{}\\{}",
                    dir_out,
                    file_name_no_ext(file_in_path))
                )
            }
            else {
                PathBuf::from(
                    &format!("{}\\{}.{}",
                    dir_out,
                    file_name_no_ext(file_in_path),
                    mta.get_ext())
                )
            };

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
    fn compress_dir(&mut self, dir_in: &Path, dir_out: &mut String) {
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
            if !self.quiet { println!("Compressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in); 
            let file_out_size = self.compress_file(file_in, &dir_out);
            if !self.quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.compress_dir(dir_in, &mut dir_out); 
        }
    }
    fn decompress_dir(&self, dir_in: &Path, dir_out: &mut String, root: bool) {
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
            if !self.quiet { println!("Decompressing {}", file_in.display()); }
            let file_in_size  = file_len(file_in);
            let file_out_size = self.decompress_file(file_in, &dir_out);
            if !self.quiet { println!("{} bytes -> {} bytes in {:.2?}\n",
                file_in_size, file_out_size, time.elapsed()); }
        }
        for dir_in in dirs.iter() {
            self.decompress_dir(dir_in, &mut dir_out, false); 
        }
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------


// Solid Archiving ------------------------------------------------------------------------------------------------------------------------
struct SolidArchiver {
    enc: Encoder,
    mta: Metadata,
    quiet: bool
}
impl SolidArchiver {
    fn new(enc: Encoder, mta: Metadata, quiet: bool) -> SolidArchiver {
        SolidArchiver {
            enc, mta, quiet,
        }
    }
    fn create_archive(&mut self) {
        for curr_file in 0..self.mta.files.len() {
            if !self.quiet { println!("Compressing {}", self.mta.files[curr_file].0); }
            let archive_size = self.compress_file_solid(curr_file);
            if !self.quiet { println!("Total archive size: {}\n", archive_size); }
        }
        self.enc.flush();
    }
    fn compress_file_solid(&mut self, curr_file: usize) -> u64 {
        // Create input file with buffer = block size
        let mut file_in = 
            new_input_file(
                self.mta.bl_sz, 
                Path::new(&self.mta.files[curr_file].0)
            );

        // Compress
        loop {
            if file_in.fill_buffer() == BufferState::Empty { break; }
            self.mta.files[curr_file].2 = file_in.buffer().len();
            self.enc.compress_block(file_in.buffer());
            self.mta.files[curr_file].1 += 1;
        }
        self.enc.file_out.stream_position().unwrap()
    }
    fn write_metadata(&mut self) {
        // Get index to end of file metadata
        self.mta.f_ptr =
        self.enc.file_out.stream_position()
        .unwrap() as usize;

        // Output number of files
        self.enc.file_out.write_usize(self.mta.files.len());

        for file in self.mta.files.iter() {
            // Get path as byte slice, truncated if longer than 255 bytes
            let path = &file.0.as_bytes()[..min(file.0.len(), 255)];

            // Output length of file path (for parsing)
            self.enc.file_out.write_byte(path.len() as u8);

            // Output path
            for byte in path.iter() {
                self.enc.file_out.write_byte(*byte);
            }

            // Output block count and final block size
            self.enc.file_out.write_usize(file.1);
            self.enc.file_out.write_usize(file.2);
        }

        // Go back to beginning of file and write header
        self.enc.file_out.rewind().unwrap();
        self.enc.write_header(&self.mta);
    }
}

struct SolidExtractor {
    dec: Decoder,
    mta: Metadata,
    quiet: bool
}
impl SolidExtractor {
    fn new(dec: Decoder, mta: Metadata, quiet: bool) -> SolidExtractor {
        SolidExtractor {
            dec, mta, quiet
        }
    }
    fn extract_archive(&mut self, dir_out: &str) {
        for curr_file in 0..self.mta.files.len() {
            if !self.quiet { println!("Decompressing {}", self.mta.files[curr_file].0); }
            self.decompress_file_solid(dir_out, curr_file);
        }
    }
    fn decompress_file_solid(&mut self, dir_out: &str, curr_file: usize) {
        let file_out_name =
            format!("{}\\{}",
                dir_out,
                file_name_ext(
                    Path::new(&self.mta.files[curr_file].0)
                ),
            );
        let mut file_out = new_output_file(4096, Path::new(&file_out_name));

        // Decompress
        for _ in 0..((self.mta.files[curr_file].1) - 1) {
            let block = self.dec.decompress_block(self.mta.bl_sz);
            for byte in block.iter() {
                file_out.write_byte(*byte);
            }
        }
        let block = self.dec.decompress_block(self.mta.files[curr_file].2);
        for byte in block.iter() {
            file_out.write_byte(*byte);
        }
        file_out.flush_buffer();
    }
    fn read_metadata(&mut self) {
        self.mta = self.dec.read_header();

        // Seek to end of file metadata
        self.dec.file_in.seek(SeekFrom::Start(self.mta.f_ptr as u64)).unwrap();
        let mut path: Vec<u8> = Vec::new();

        // Get number of files
        let num_files = self.dec.file_in.read_usize();

        // Parse files and lengths
        for _ in 0..num_files {
            // Get length of next path
            let len = self.dec.file_in.read_byte();

            // Get path, block count, final block size
            for _ in 0..len {
                path.push(self.dec.file_in.read_byte());
            }
            self.mta.files.push(
                (path.iter().cloned()
                    .map(|b| b as char)
                    .collect::<String>(),
                    self.dec.file_in.read_usize(),
                    self.dec.file_in.read_usize())
            );
            path.clear();
        }

        // Seek back to beginning of compressed data
        self.dec.file_in.seek(SeekFrom::Start(40)).unwrap();

        self.dec.init_x();
    }
}
// ----------------------------------------------------------------------------------------------------------------------------------------

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

enum Parse {
    Mode,
    DirOut,
    Solid,
    Sort,
    Inputs,
    Quiet,
}

fn main() {
    let args = env::args().skip(1);

    if args.len() == 0 {
        print_program_info();
    }

    #[allow(unused_assignments)]
    let mut dir_out = String::new();

    // Initialize settings
    let mut parser = Parse::Mode;
    let mut solid = false;
    let mut sort = Sort::None;
    let mut user_out = String::new();
    let mut mode = "c";
    let mut quiet = false;
    let mut inputs: Vec<String> = Vec::new();

    for arg in args {
        match arg.as_str() {
            "-out" => {
                parser = Parse::DirOut;
                continue;
            },     
            "-sort" => {
                parser = Parse::Sort;
                continue;
            }, 
            "-i" => {
                parser = Parse::Inputs;
                continue;
            },
            "-sld" => {
                parser = Parse::Solid;
            },
            "-q" => {
                parser = Parse::Quiet;
            }
            "-help" => {
                print_program_info();
            }
            _ => {},
        }
        match parser {
            Parse::Mode => {
                match arg.as_str() {
                    "c" => mode = "c",
                    "d" => mode = "d",
                    _ => {
                        println!("Error.");
                        std::process::exit(1);
                    }
                }
            }
            Parse::DirOut => {
                user_out = arg;
            }
            Parse::Sort => {
                sort = match arg.as_str() {
                    "ext"    => Sort::Ext,
                    "prtdir" => Sort::PrtDir,
                    "crtd"   => Sort::Created,
                    "accd"   => Sort::Accessed,
                    "mod"    => Sort::Modified,
                    _ => { 
                        println!("No valid sort criteria found.");
                        std::process::exit(1);
                    }
                }
            }
            Parse::Inputs => {
                inputs.push(arg);
            }
            Parse::Solid => {
                solid = true;
            }
            Parse::Quiet => {
                quiet = true;
            }
        }
    }

    // Filter invalid inputs
    let inputs: Vec<PathBuf> = 
        inputs.iter()
        .map(PathBuf::from)
        .filter(|i| i.is_file() || i.is_dir())
        .collect();
    
    // Format output directory
    dir_out = match mode {
        "c" => {
            if solid { format_dir_out(Format::ArchiveSolid, &user_out, &inputs[0]) }
            else     { format_dir_out(Format::Archive,      &user_out, &inputs[0]) }
        }
        "d" => {
            if solid { format_dir_out(Format::ExtractSolid, &user_out, &inputs[0]) }
            else     { format_dir_out(Format::Extract,      &user_out, &inputs[0]) }
        }
        _ => String::new(),
    };
    
    if !quiet {
        println!();
        println!("//////////////////////////////////////////////////////////////");
        println!(
            "Creating {} archive {} of inputs:\n{:#?},\nsorting by {}.",
            if solid { "solid" } else { "non-solid" },
            dir_out, 
            inputs,
            match sort {
                Sort::None     => "none",
                Sort::Ext      => "extension",
                Sort::PrtDir   => "parent directory",
                Sort::Created  => "creation time",
                Sort::Accessed => "last accessed time",
                Sort::Modified => "last modified time",
            }
        );
        println!("//////////////////////////////////////////////////////////////");
        println!();
    }

    if solid {
        let mut mta: Metadata = Metadata::new();
        match mode {
            "c" => {
                // Sort files and directories
                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) =
                    inputs.into_iter().partition(|f| f.is_file());

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

                let enc = Encoder::new(new_output_file(4096, Path::new(&dir_out)));
                let mut sld_arch = SolidArchiver::new(enc, mta, quiet);

                sld_arch.create_archive();
                sld_arch.write_metadata();    
            }
            "d" => {
                new_dir(&dir_out);

                let dec = Decoder::new(new_input_file(4096, &inputs[0]));
                let mut sld_extr = SolidExtractor::new(dec, mta, quiet);

                sld_extr.read_metadata();
                sld_extr.extract_archive(&dir_out);    
            }
            _ => {
                println!("Couldn't parse input. For help, type PROG_NAME.");
            }
        }
    }
    else {
        let mut arch = Archiver::new(quiet);
        match mode {
            "c" => {
                new_dir(&dir_out);

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    inputs.into_iter().partition(|f| f.is_file());

                for file_in in files.iter() {
                    let time = Instant::now();
                    if !quiet { println!("Compressing {}", file_in.display()); }
                    let file_in_size  = file_len(file_in); 
                    let file_out_size = arch.compress_file(file_in, &dir_out);
                    if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                        file_in_size, file_out_size, time.elapsed()); }
                }
                for dir_in in dirs.iter() {
                    arch.compress_dir(dir_in, &mut dir_out);      
                }
            }
            "d" => {
                new_dir(&dir_out);

                let (files, dirs): (Vec<PathBuf>, Vec<PathBuf>) = 
                    inputs.into_iter().partition(|f| f.is_file());

                for file_in in files.iter() {
                    let time = Instant::now();
                    if !quiet { println!("Decompressing {}", file_in.display()); }
                    let file_in_size  = file_len(file_in); 
                    let file_out_size = arch.decompress_file(file_in, &dir_out);
                    if !quiet { println!("{} bytes -> {} bytes in {:.2?}\n", 
                        file_in_size, file_out_size, time.elapsed()); } 
                }
                for dir_in in dirs.iter() {
                    arch.decompress_dir(dir_in, &mut dir_out, true);      
                }
            }
            _ => {
                println!("Couldn't parse input. For help, type PROG_NAME.");
            }
        }
    }     
}
