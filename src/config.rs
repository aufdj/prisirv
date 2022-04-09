use std::path::PathBuf;

use crate::{
    sort::Sort, Mode, fv,
    formatting::fmt_root_output_dir,
    extract::Extractor,
    error,
    block::Block,
    metadata::FileData,
};


/// An enum containing each possible parsing state.
enum Parse {
    None,
    Compress,
    Decompress,
    DirOut,
    Sort,
    Inputs,
    Quiet,
    Clobber,
    Mem,
    Lvl,
    BlkSz,
    Threads,
    List,
    Fv,
    Align,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Align {
    File,
    Exact,
}

/// A list of all user defined configuration settings.
#[derive(Clone, Debug)]
pub struct Config {
    pub sort:      Sort,          // Sorting method (solid archives only)
    pub user_out:  String,        // User specified output directory (optional)
    pub dir_out:   String,        // Output directory
    pub inputs:    Vec<FileData>, // Inputs to be archived or extracted
    pub quiet:     bool,          // Suppresses output other than errors
    pub mode:      Mode,          // Compress or decompress
    pub mem:       u64,           // Memory usage 
    pub clbr:      bool,          // Allow clobbering files
    pub blk_sz:    usize,         // Block size
    pub threads:   usize,         // Maximum number of threads
    pub align:     Align,         // Block size exactly as specified or extended to next file boundary
}
impl Config {
    /// Create a new default Config.
    pub fn default() -> Config {
        Config {
            sort:      Sort::None,
            user_out:  String::new(),
            blk_sz:    10 << 20,
            mem:       1 << 22,
            mode:      Mode::Compress,
            quiet:     false,
            clbr:      false,
            threads:   4,
            inputs:    Vec::new(),
            dir_out:   String::new(),
            align:     Align::Exact,
        }
    }
    /// Create a new Config with the specified command line arguments.
    pub fn new(args: &[String]) -> Config {
        if args.is_empty() { print_program_info(); }

        let mut parser = Parse::None;
        let mut cfg    = Config::default();
        let mut list   = false;
        let mut fv     = false;
        let mut cs     = 10.0;
        
        for arg in args.iter() {
            match arg.as_str() {
                "-sort" => {
                    parser = Parse::Sort;
                    continue;
                }, 
                "-out" | "-output-path" => {
                    parser = Parse::DirOut;
                    continue;
                },     
                "-i" | "-inputs" => { 
                    parser = Parse::Inputs;
                    continue;
                },
                "-mem" | "-memory" => {
                    parser = Parse::Mem;
                    continue;
                }
                "-blk" | "-block-size" => {
                    parser = Parse::BlkSz;
                    continue;
                } 
                "-threads" => {
                    parser = Parse::Threads;
                    continue;
                }
                "fv" => {
                    parser = Parse::Fv;
                    continue;
                }
                "create"            => parser = Parse::Compress,
                "extract"           => parser = Parse::Decompress,
                "-q"   | "-quiet"   => parser = Parse::Quiet,
                "-clb" | "-clobber" => parser = Parse::Clobber,
                "-file-align"       => parser = Parse::Align,
                "ls" | "list"       => parser = Parse::List,
                "help"              => print_program_info(),
                _ => {},
            }
            match parser {
                Parse::Sort => {
                    cfg.sort = match arg.as_str() {
                        "ext"  => Sort::Ext,
                        "name" => Sort::Name,
                        "len"  => Sort::Len,
                        "crtd" => Sort::Created,
                        "accd" => Sort::Accessed,
                        "mod"  => Sort::Modified,
                        "prt"  => {
                            parser = Parse::Lvl;
                            Sort::PrtDir(1)
                        },
                        m => { error::invalid_sort_criteria(m); }
                    }
                }
                Parse::Lvl => {
                    match arg.parse::<usize>() {
                        Ok(lvl) => cfg.sort = Sort::PrtDir(lvl),
                        Err(_)  => { error::invalid_lvl(); }
                    }
                }
                Parse::Mem => {
                    // Parse memory option (0..9)
                    cfg.mem = match arg.parse::<u64>() {
                        Ok(opt) => match opt {
                            0..=9 => 1 << (20 + opt),
                            _ => error::out_of_range_memory_option(opt),
                        }
                        Err(_) => error::invalid_memory_option(),
                    };
                } 
                Parse::BlkSz => {
                    let scale = 
                    if      arg.contains('B') { 1 }
                    else if arg.contains('K') { 1024 }
                    else if arg.contains('M') { 1024*1024 }
                    else if arg.contains('G') { 1024*1024*1024 }
                    else { error::invalid_scale(); };

                    cfg.blk_sz = 
                    match arg.chars().filter(|s| s.is_numeric())
                    .collect::<String>().parse::<usize>() {
                        Ok(size) => size * scale,
                        Err(_)   => error::invalid_block_size(),
                    }
                }
                Parse::Threads => {
                    cfg.threads = match arg.parse::<usize>() {
                        Ok(count) => match count {
                            0..=128 => count,
                            _ => error::max_thread_count(count),
                        }
                        Err(_) => error::invalid_thread_count(),
                    };
                }
                Parse::List => {
                    list = true; 
                    parser = Parse::Inputs;
                }
                Parse::Fv => {
                    fv = true;
                    match arg.parse::<f64>() {
                        Ok(c) => {
                            cs = c;
                            parser = Parse::Inputs;
                        }
                        Err(_) => cfg.inputs.push(FileData::new(PathBuf::from(arg))),
                    }
                }
                Parse::Compress   => cfg.mode = Mode::Compress,
                Parse::Decompress => cfg.mode = Mode::Decompress,
                Parse::DirOut     => cfg.user_out = arg.to_string(),
                Parse::Inputs     => cfg.inputs.push(FileData::new(PathBuf::from(arg))),
                Parse::Quiet      => cfg.quiet = true,
                Parse::Clobber    => cfg.clbr = true,
                Parse::Align      => cfg.align = Align::File,
                Parse::None => {},
            }
        }

        if cfg.inputs.is_empty() { error::no_inputs(); }

        for input in cfg.inputs.iter() {
            if !(input.path.is_file() || input.path.is_dir()) {
                error::invalid_input(&input.path);
            }
        }

        if fv { fv::fv(&cfg.inputs[0].path, cs); }

        cfg.dir_out = fmt_root_output_dir(&cfg);

        if list { cfg.list_archive(); }
        cfg.print();
        cfg
    }

    /// Print information about the current Config.
    pub fn print(&self) {
        if !self.quiet {
            println!();
            println!("=======================================================================");

            println!(" {} Archive of Inputs:", 
                if self.mode == Mode::Compress { "Creating" } else { "Extracting" },
            );

            for input in self.inputs.iter() {
                println!("    {} ({})", 
                    input.path.display(),
                    if input.path.is_file() { "File" }
                    else if input.path.is_dir() { "Directory" }
                    else { "" }
                );
            }
            println!();

            println!(" Output Path: {}", self.dir_out);
            if self.mode == Mode::Compress {
                println!(" Sorting by: {}", 
                match self.sort {
                    Sort::None     => "None",
                    Sort::Ext      => "Extension",
                    Sort::Name     => "Name",
                    Sort::Len      => "Length",
                    Sort::Created  => "Creation time",
                    Sort::Accessed => "Last accessed time",
                    Sort::Modified => "Last modified time",
                    Sort::PrtDir(_) => "Parent Directory",
                });

                println!(" Memory Usage: {} MiB", 3 + (self.mem >> 20) * 3);

                let (size, suffix) = format(self.blk_sz);
                println!(" Block Size: {} {}", size, suffix); 

                println!(" Block alignment: {}", 
                    if self.align == Align::File { "File" } 
                    else { "Exact" }
                );
            }

            println!(" Threads: Up to {}", self.threads);

            println!("=======================================================================");
            println!();
        }
    }

    fn list_archive(self) -> ! {
        let mut blk = Block::new(self.blk_sz);
        let mut extr = Extractor::new(self); 
        for _ in 0..extr.mta.blk_c {
            blk.read_from(&mut extr.archive);
            blk.print();
            blk.next();
        }
        std::process::exit(0);
    }
}


/// Print information about Prisirv.
fn print_program_info() {
    println!();
    println!("     ______   ______     ________  ______    ________  ______    __   __     
    /_____/\\ /_____/\\   /_______/\\/_____/\\  /_______/\\/_____/\\  /_/\\ /_/\\    
    \\:::_ \\ \\\\:::_ \\ \\  \\__.::._\\/\\::::_\\/_ \\__.::._\\/\\:::_ \\ \\ \\:\\ \\\\ \\ \\   
     \\:(_) \\ \\\\:(_) ) )_   \\::\\ \\  \\:\\/___/\\   \\::\\ \\  \\:(_) ) )_\\:\\ \\\\ \\ \\  
      \\: ___\\/ \\: __ `\\ \\  _\\::\\ \\__\\_::._\\:\\  _\\::\\ \\__\\: __ `\\ \\\\:\\_/.:\\ \\ 
       \\ \\ \\    \\ \\ `\\ \\ \\/__\\::\\__/\\ /____\\:\\/__\\::\\__/\\\\ \\ `\\ \\ \\\\ ..::/ / 
        \\_\\/     \\_\\/ \\_\\/\\________\\/ \\_____\\/\\________\\/ \\_\\/ \\_\\/ \\___/_(  
                                                                             ");
    println!("  
      Prisirv, Context Mixing File Archiver
      Copyright (C) 2022 aufdj
      
      This program is free software: you can redistribute it and/or modify
      it under the terms of the GNU General Public License as published by
      the Free Software Foundation, either version 3 of the License, or
      (at your option) any later version.
      
      This program is distributed in the hope that it will be useful,
      but WITHOUT ANY WARRANTY; without even the implied warranty of
      MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
      GNU General Public License for more details.
      
      You should have received a copy of the GNU General Public License
      along with this program.  If not, see <https://www.gnu.org/licenses/>.");
    println!("
      Source code available at https://github.com/aufdj/prisirv");
    println!();
    println!();
    println!("  USAGE: PROG_NAME [c|d] [-i [..]] [OPTIONS|FLAGS]");
    println!();
    println!("  REQUIRED:");
    println!("     create                Create archive");
    println!("     extract               Extract archive");
    println!("    -i,     -inputs        Specify list of input files/dirs");
    println!();
    println!("  OPTIONS:");
    println!("    -out,   -output-path   Specify output path");
    println!("    -mem,   -memory        Specify memory usage             (Default - 15 MiB)");
    println!("    -blk,   -block-size    Specify block size               (Default - 10 MiB)");
    println!("    -threads               Specify thread count             (Default - 4)");
    println!("    -sort                  Sort files (solid archives only) (Default - none)");
    println!();
    println!("  FLAGS:");
    println!("    -sld,   -solid         Create solid archive");
    println!("    -q,     -quiet         Suppresses output other than errors");
    println!("    -clb,   -clobber       Allows clobbering files");
    println!("    -file-align            Extends blocks to end of current file");
    println!();
    println!("  Sorting Methods:");
    println!("      -sort ext      Sort by extension");
    println!("      -sort name     Sort by name");
    println!("      -sort len      Sort by length");
    println!("      -sort prt n    Sort by nth parent directory");
    println!("      -sort crtd     Sort by creation time");
    println!("      -sort accd     Sort by last access time");
    println!("      -sort mod      Sort by last modification time");
    println!();
    println!("  Memory Options:");
    println!("      -mem 0  6 MB   -mem 5  99 MB");
    println!("      -mem 1  9 MB   -mem 6  195 MB");
    println!("      -mem 2  15 MB  -mem 7  387 MB");
    println!("      -mem 3  27 MB  -mem 8  771 MB");
    println!("      -mem 4  51 MB  -mem 9  1539 MB");
    println!();
    println!("  Decompression requires same memory option used for compression.");
    println!("  Any memory option specified for decompression will be ignored.");
    println!();
    println!("  EXAMPLE:");
    println!("      Compress file [\\foo\\bar.txt] and directory [\\baz] into solid archive [\\foo\\arch], \n      sorting files by creation time:");
    println!();
    println!("          prisirv c -i \\foo\\bar.txt \\baz -out arch -sld -sort crtd ");
    println!();
    println!("      Decompress the archive:");
    println!();
    println!("          prisirv d -i \\foo\\arch.prsv -sld ");
    std::process::exit(0);
}

fn format(size: usize) -> (usize, String) {
    if size >= 1024*1024*1024 {
        (size/1024/1024/1024, String::from("GiB"))
    }
    else if size >= 1024*1024 {
        (size/1024/1024, String::from("MiB"))
    }
    else if size >= 1024 {
        (size/1024, String::from("KiB"))
    }
    else if size >= 1 {
        (size, String::from("B"))
    }
    else { (0, String::from("")) }
}

