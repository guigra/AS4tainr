**FREE
// Stage 2, S2.120: WRITE and UPDATE from a result data structure, reads
// into one. Captured on PUB400 (tools/golden/native120.txt n120_dsio)
// (the qualified form; the native unit NDSIO takes gn120du.rpgle).
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*update:*output) qualified extfile('GUIGRA261/GN120F');
dcl-ds o likerec(gn120f.gn120f:*output);
dcl-ds r likerec(gn120f.gn120f:*input);
dcl-s line varchar(80);

o.n = 5;
o.s = 'abc';
write gn120f.gn120f o;
o.n = 6;
o.s = 'de';
write gn120f.gn120f o;
close gn120f;
open gn120f;
read gn120f r;
r.n = r.n * 10;
r.s = 'zz';
update gn120f.gn120f r;
close gn120f;
open gn120f;
read gn120f r;
dow not %eof(gn120f);
  line += %char(r.n) + ':' + %trimr(r.s) + ' ';
  read gn120f r;
enddo;
say(%trimr(line));
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
