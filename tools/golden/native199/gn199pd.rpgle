**FREE
// Stage 2, S2.199: the difference of two pointers in %CHAR: a pointer
// field minus %ADDR of a field, and %ADDR of one array element minus
// %ADDR of another. Captured on PUB400 (tools/golden/native199.txt
// n199_ptrdiff) for the native unit NPTRDF2.
ctl-opt dftactgrp(*no);
dcl-s name char(10) inz('abcdefghij');
dcl-s p pointer;
dcl-s nums int(10) dim(3);
dcl-s line varchar(200);

p = %addr(name);
p += 2;
line = %char(p - %addr(name)) + ' ' + %char(%addr(nums(3)) - %addr(nums(1)))
     + ' ' + %char(%addr(name) - p);
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
