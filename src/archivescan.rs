use crate::{
    block::Block,
    filedata::FileData,
    buffered_io::new_input_file,
};

// Count number of blocks in an archive.
pub fn block_count(ex_arch: &FileData) -> usize {
    let mut count = 0;
    let mut blk = Block::default();
    let mut archive = new_input_file(4096, &ex_arch.path);
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 { 
            return count; 
        }
        count += 1;
        blk.next();
    }
}

// Print archive information.
pub fn list_archive(ex_arch: &FileData) -> ! {
    let mut blk = Block::default();
    let mut archive = new_input_file(4096, &ex_arch.path);
    loop {
        blk.read_header_from(&mut archive);
        if blk.sizeo == 0 { 
            break; 
        }
        blk.print();
        blk.next();
    }
    std::process::exit(0);
}