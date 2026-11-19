**FREE
// Stage 2, S2.128: OPEN and CLOSE of a database file: a user-opened file
// read, opened again while open and closed twice under MONITOR, then
// opened again. GN120F is native120's table (50 zz, 6 de). Captured on
// PUB400 (tools/golden/native128.txt n128_openclose) for the native unit
// NOPNCLS.
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*input) usropn qualified extfile('GUIGRA261/GN120F');
dcl-ds r likerec(gn120f.gn120f);
dcl-s line varchar(80);

open gn120f;
read gn120f r;
line = %char(r.n);
monitor;
  open gn120f;
  line += ' again';
on-error;
  line += ' ' + %char(%status);
endmon;
close gn120f;
monitor;
  close gn120f;
  line += ' closed';
on-error;
  line += ' ' + %char(%status);
endmon;
open gn120f;
read gn120f r;
line += ' ' + %char(r.n);
close gn120f;
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
