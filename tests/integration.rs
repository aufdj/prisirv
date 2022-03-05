
#[cfg(test)]
mod tests {
    use prisirv::Prisirv;
    use prisirv::crc32::crc32;
    use std::path::Path;

    #[test]
    fn test() {
        let inputs: Vec<&str>  = vec!["tests\\data\\calgary.tar"];
        Prisirv::new().solid().clobber().create_archive_of(&inputs);

        let outputs: Vec<&str> = vec!["tests\\data\\calgary.prsv"];
        Prisirv::new().solid().clobber().extract_archive_of(&outputs);
        
        let crc1 = crc32(Path::new("tests\\data\\calgary.tar"));
        let crc2 = crc32(Path::new("tests\\data\\calgary_d\\calgary.tar"));

        println!("Input crc: {}", crc1);
        println!("Output crc: {}", crc2);
        
        assert!(crc1 == crc2);
    }
    
}