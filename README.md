# prisirv

<pre>
USAGE: PROG_NAME [REQUIRED] [OPTIONS|FLAGS]
    
REQUIRED:
   c,  create            Create archive
   x,  extract           Extract archive
   a,  append            Append files to archive
   p,  pick              Extract select files from archive
   m,  merge             Merge archives together
   ls                    List info about archive
        
One of the above commands must be used, and all are mutually exclusive.
        
OPTIONS:
  -i,    -inputs         Specify list of input files/dirs
  -out,  -output-path    Specify output path
  -mem,  -memory         Specify memory usage   (Default - 2 (15 MiB))
  -blk,  -block-size     Specify block size     (Default - 10 MiB)
  -threads               Specify thread count   (Default - 4)
  -sort                  Sort files             (Default - none)
        
Options '-memory', '-block-size', and '-sort' have no effect on extraction.
        
FLAGS:
  -q,  -quiet            Suppresses output other than errors
  -clobber               Allow file clobbering
  -file-align            Truncate blocks to align with file boundaries
  -store                 Store files with no compression
        
Flags '-file-align' and '-store' have no effect on extraction.
        
Sorting Methods:
  -sort ext      Sort by extension
  -sort name     Sort by name
  -sort len      Sort by length
  -sort prt n    Sort by nth parent directory
  -sort crtd     Sort by creation time
  -sort accd     Sort by last access time
  -sort mod      Sort by last modification time
        
Memory Options:
  -mem 0  6 MB   -mem 5  99 MB
  -mem 1  9 MB   -mem 6  195 MB
  -mem 2  15 MB  -mem 7  387 MB
  -mem 3  27 MB  -mem 8  771 MB
  -mem 4  51 MB  -mem 9  1539 MB


EXAMPLES:
        
Compress file [/foo/bar.txt] and directory [/baz] into archive [/foo/qux.prsv], 
sorting files by creation time:
       
    prisirv create -inputs /foo/bar.txt /baz -sort crtd -output-path qux
       
Extract archive [/foo/qux.prsv]:
       
    prisirv extract /foo/qux.prsv
       
Append file [foo.txt] to archive [/foo/qux.prsv]:
       
    prisirv append /foo/qux.prsv -inputs foo.txt
       
Extract file [foo.txt] from archive [/foo/qux.prsv]:
       
    prisirv pick /foo/qux.prsv -inputs foo.txt

Merge archives [archive2.prsv] and [archive3.prsv] into [archive1.prsv]:

    prisirv merge archive1.prsv -inputs archive2.prsv archive3.prsv
       
List information about archive [/foo/qux.prsv]:
       
    prisirv ls /foo/qux.prsv
</pre>
