**FREE
// Stage 2, S2.123: %CHAR of a comparison, of AND and OR, and of NOT.
// Captured on PUB400 (tools/golden/native123.txt n123_charcond) for the
// native unit NCHARCND.
ctl-opt dftactgrp(*no);
dcl-s n int(10) inz(5);
dcl-s c char(3) inz('abc');
dcl-s line varchar(80);

line = %char(n > 3) + %char(n = 3 or n < 0) + %char(not (n > 3))
     + %char(n >= 5 and n < 10) + %char(c < 'abd') + %char(c <> 'abc');
line += ' ' + %char(n * 2 = 10);
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
