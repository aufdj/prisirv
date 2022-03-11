
#[cfg(test)]
mod tests {
    use prisirv::Prisirv;
    use prisirv::crc32::crc32;
    use std::{fs, path::Path};

    #[test]
    #[ignore]
    fn solid_archive_calgary_tar() {
        let inputs: Vec<&str> = vec!["tests\\data\\calgary.tar"];
        Prisirv::new().solid().clobber().create_archive_of(&inputs);

        let inputs: Vec<&str> = vec!["tests\\data\\calgary.prsv"];
        Prisirv::new().solid().clobber().extract_archive_of(&inputs);
        
        let crc1 = crc32(Path::new("tests\\data\\calgary.tar"));
        let crc2 = crc32(Path::new("tests\\data\\calgary_d\\tests\\data\\calgary.tar"));

        fs::remove_dir_all("tests\\data\\calgary_d").unwrap();
        fs::remove_file("tests\\data\\calgary.prsv").unwrap();

        println!("Input CRC:  {:x}", crc1);
        println!("Output CRC: {:x}", crc2);
        println!();
        
        assert!(crc1 == crc2);
    }  
}
