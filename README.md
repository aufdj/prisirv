# prisirv

Prisirv is a context mixing archiver based on [lpaq1 by Matt Mahoney.](http://mattmahoney.net/dc/#lpaq)

<pre>
USAGE: PROG_NAME [create|extract] [-inputs [..]] [OPTIONS|FLAGS]

REQUIRED: 
    create               Create archive
    extract              Extract archive
   -i,    -inputs        Specify list of inputs

OPTIONS:
   -out,  -output-path   Specify output path
   -mem,  -memory        Specify memory usage   (Default - 2 (15 MiB))
   -blk,  -block-size    Specify block size     (Default - 10 MiB)
   -threads              Specify thread count   (Default - 4)
   -sort                 Sort files             (Default - None)

FLAGS:
   -q,    -quiet         Suppress output other than errors
   -clobber              Allow file clobbering
   -file-align           Truncate blocks to align with files
   -lzw                  Use LZW compression method

Sorting Methods:
   -sort ext      Sort by extension
   -sort name     Sort by name
   -sort len      Sort by length
   -sort prt n    Sort by nth parent directory
   -sort crtd     Sort by creation time
   -sort accd     Sort by last access time
   -sort mod      Sort by last modification time

Any sorting option specified for extraction will be ignored.
  
Memory Options:
   -mem 0  6 MiB   -mem 5  99 MiB
   -mem 1  9 MiB   -mem 6  195 MiB
   -mem 2  15 MiB  -mem 7  387 MiB
   -mem 3  27 MiB  -mem 8  771 MiB
   -mem 4  51 MiB  -mem 9  1539 MiB

Extraction requires same memory option used for archiving.
Any memory option specified for extraction will be ignored.
  
EXAMPLE:
  Compress file [\foo\bar.txt] and directory [\baz] into archive [\foo\arch],
  sorting files by creation time:

    prisirv create -inputs \foo\bar.txt \baz -output-path arch -sort crtd

  Decompress the archive:

    prisirv extract -inputs \foo\arch.prsv
</pre>
