use std::{
    path::Path,
    cmp::Ordering,
    ffi::OsString,
};

use crate::{
    error::SortError,
    filedata::{FileData, Type},
};

/// Sort files to improve compression of solid archives.

/// Possible sorting methods.
#[derive(Debug, Clone, Copy)]
pub enum Sort {    // Sort By:
    None,          // No sorting
    Ext,           // Extension
    Name,          // Name
    Len,           // Length
    PrtDir(usize), // Parent Directory
    Created,       // Creation Time
    Accessed,      // Last Access Time
    Modified,      // Last Modification Time
}

/// Sort files by given sorting method.
pub fn sort_files(f1: &FileData, f2: &FileData, sorting_method: Sort) -> Result<Ordering, SortError> {
    // Move compressed files to end of inputs list.
    if f1.kind == Type::Compressed {
        return Ok(Ordering::Greater);
    }
    if f2.kind == Type::Compressed {
        return Ok(Ordering::Less);
    }
    match sorting_method {
        Sort::Ext => {
            let ext1 = match f1.path.extension() {
                Some(ext) => ext.to_ascii_lowercase(),
                None => OsString::new(),
            };
            let ext2 = match f2.path.extension() {
                Some(ext) => ext.to_ascii_lowercase(),
                None => OsString::new(),
            };
            Ok((ext1).cmp(&ext2))
        }
        Sort::Name => {
            let name1 = match f1.path.file_name() {
                Some(name) => name.to_ascii_lowercase(),
                None => OsString::new(),
            };
            let name2 = match f2.path.file_name() {
                Some(name) => name.to_ascii_lowercase(),
                None => OsString::new(),
            };
            Ok((name1).cmp(&name2))
        }
        Sort::Len => {
            let len1 = match f1.path.metadata() {
                Ok(data) => data.len(),
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }     
            };
            let len2 = match f2.path.metadata() {
                Ok(data) => data.len(),
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }     
            };
            Ok((len1).cmp(&len2))
        }
        Sort::PrtDir(lvl) => {
            let parent1 = match f1.path.ancestors().nth(lvl) {
                Some(path) => path,
                None => Path::new(""),
            };
            let parent2 = match f2.path.ancestors().nth(lvl) {
                Some(path) => path,
                None => Path::new(""),
            };
            Ok((parent1).cmp(parent2))
        }
        Sort::Created => {
            let creation_time1 = match f1.path.metadata() {
                Ok(data) => {
                    match data.created() {
                        Ok(creation_time) => creation_time,
                        Err(_) => {
                            return Err(SortError::CreationTimeNotSupported);
                        }
                    }
                }  
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }
            };
            let creation_time2 = match f2.path.metadata() {
                Ok(data) => {
                    match data.created() {
                        Ok(creation_time) => creation_time,
                        Err(_) => {
                            return Err(SortError::CreationTimeNotSupported);
                        }
                    }
                }  
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }
            };
            Ok((creation_time1).cmp(&creation_time2))
        }
        Sort::Accessed => {
            let access_time1 = match f1.path.metadata() {
                Ok(data) => {
                    match data.accessed() {
                        Ok(access_time) => access_time,
                        Err(_) => {
                            return Err(SortError::AccessTimeNotSupported);
                        }
                    }
                }  
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }
            };
            let access_time2 = match f2.path.metadata() {
                Ok(data) => {
                    match data.accessed() {
                        Ok(access_time) => access_time,
                        Err(_) => {
                            return Err(SortError::AccessTimeNotSupported);
                        }
                    }
                }  
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }
            };
            Ok((access_time1).cmp(&access_time2))
        }
        Sort::Modified => {
            let modified_time1 = match f1.path.metadata() {
                Ok(data) => {
                    match data.modified() {
                        Ok(modified_time) => modified_time,
                        Err(_) => {
                            return Err(SortError::ModifiedTimeNotSupported);
                        }
                    }
                }  
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }
            };
            let modified_time2 = match f2.path.metadata() {
                Ok(data) => {
                    match data.modified() {
                        Ok(modified_time) => modified_time,
                        Err(_) => {
                            return Err(SortError::ModifiedTimeNotSupported);
                        }
                    }
                }  
                Err(_) => {
                    return Err(SortError::MetadataNotSupported);
                }
            };
            Ok((modified_time1).cmp(&modified_time2))
        }
        Sort::None => Ok(Ordering::Equal),
    }
}