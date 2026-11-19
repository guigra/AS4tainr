**FREE
// Stage 2, S2.131: IN %RANGE and IN %LIST in conditions, of numbers and
// characters, with expressions. Captured on PUB400
// (tools/golden/native131.txt n131_inlist) for the native unit NINLIST.
ctl-opt dftactgrp(*no);
dcl-s n int(10) inz(5);
dcl-s c char(1) inz('G');
dcl-s line varchar(80);

if n in %range(1 : 10);
  line += 'a';
endif;
if n in %range(6 : 10);
  line += 'b';
endif;
if c in %list('A' : 'G' : 'Z');
  line += 'c';
endif;
if not (c in %list('A' : 'B'));
  line += 'd';
endif;
if n * 2 in %list(3 : 10);
  line += 'e';
endif;
if 'M' in %range('A' : 'Z');
  line += 'f';
endif;
if n in %range(5 : 5);
  line += 'g';
endif;
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
