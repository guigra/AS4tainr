**FREE
// Stage 2, S2.120: as gn120ds.rpgle with the file not qualified (its
// format renamed): WRITE and UPDATE from a result data structure, reads
// into one. Captured on PUB400 (tools/golden/native120.txt n120_dsiou)
// for the native unit NDSIO.
ctl-opt dftactgrp(*no);
dcl-f gn120f usage(*update:*output) rename(gn120f:rec120)
             extfile('GUIGRA261/GN120F');
dcl-ds o likerec(rec120:*output);
dcl-ds r likerec(rec120:*input);
dcl-s line varchar(80);

o.n = 5;
o.s = 'abc';
write rec120 o;
o.n = 6;
o.s = 'de';
write rec120 o;
close gn120f;
open gn120f;
read gn120f r;
r.n = r.n * 10;
r.s = 'zz';
update rec120 r;
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
