
#[cfg(test)]
mod tests {
    use prisirv::Prisirv;
    use prisirv::crc32::Crc32;
    use std::{fs, path::Path};

    #[test]
    fn calgary_tar() {
        let inputs: Vec<&str> = vec!["tests\\data\\calgary.tar"];
        if let Err(err) = Prisirv::default().clobber().inputs(&inputs).create_archive() {
            println!("{err}");
        }

        let inputs: Vec<&str> = vec!["tests\\data\\calgary.prsv"];
        if let Err(err) = Prisirv::default().clobber().inputs(&inputs).extract_archive() {
            println!("{err}");
        }
        
        let crc1 = Path::new("tests\\data\\calgary.tar").crc32();
        let crc2 = Path::new("tests\\data\\calgary_d\\tests\\data\\calgary.tar").crc32();

        fs::remove_dir_all("tests\\data\\calgary_d").unwrap();
        fs::remove_file("tests\\data\\calgary.prsv").unwrap();

        println!();
        println!("Input CRC:  {:x}", crc1);
        println!("Output CRC: {:x}", crc2);
        println!();
        
        assert!(crc1 == crc2);
    }
}
