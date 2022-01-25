# prisirv

Prisirv is a context mixing archiver based on lpaq1.

<pre>
USAGE: PROG_NAME [c|d] [-sld] [-sort [..]] [files|dirs]

OPTIONS:
   c       Compress
   d       Decompress
  -sld     Create solid archive
  -sort    Sort files (solid archives only)

Sorting Methods:
  ext      Sort by extension
  prtdir   Sort by parent directory
  crtd     Sort by creation time
  accd     Sort by last access time
  mod      Sort by last modification time
  
EXAMPLE:
  Compress file [\foo\bar.txt] and directory [baz] into solid archive, 
  sorting files by creation time:

    prisirv c -sld -sort crtd \foo\bar.txt \baz

  Decompress the archive:

    prisirv d -s \foo.lpaq
</pre>

