**FREE
// Interpreter fix (fork branch fix/sqrt-places): %CHAR of %SQRT of an
// integer that is a square, of 10 and 2, and of a packed(5:2) value.
// Captured on PUB400 (tools/golden/native213.txt n213_sqrt).
ctl-opt dftactgrp(*no);
dcl-s p packed(5:2) inz(6.25);
dcl-s i int(10) inz(16);
dcl-s line varchar(300);

line = %char(%sqrt(16)) + '|' + %char(%sqrt(10)) + '|' + %char(%sqrt(2))
       + '|' + %char(%sqrt(p)) + '|' + %char(%sqrt(i));
say(line);
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(300) const;
  end-pi;
  dcl-s wbuf varchar(300);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
