**FREE
// Stage 2, S2.94: CLEAR of a character, packed and varying field, an
// array, a data structure (character, zoned and binary subfields, one
// overlaying another), a qualified one, an indicator, *IN, and in a
// procedure a local field and a local data structure. The values after
// each CLEAR, with x'00' shown as 0. Captured on PUB400
// (tools/golden/native94.txt n94_clear) for the native unit NCLEAR.
ctl-opt dftactgrp(*no);
dcl-s c char(4) inz('abcd');
dcl-s p packed(5:2) inz(12.34);
dcl-s v varchar(6) inz('xyz');
dcl-s arr int(5) dim(3) inz(7);
dcl-ds d;
  dc char(3) inz('QRS');
  dz zoned(3:0) inz(42);
  db int(10) inz(9);
  dall char(4) pos(1);
end-ds;
dcl-ds q qualified;
  a char(2) inz('mn');
  n packed(3:0) inz(5);
end-ds;
dcl-s line varchar(120);
*in31 = *on;
*in32 = *on;
clear c;
clear p;
clear v;
clear arr;
clear d;
clear q;
clear *in31;
line = '[' + c + '] ' + %char(p) + ' ' + %char(%len(v)) + ' '
       + %char(arr(1) + arr(3)) + ' [' + dc + '] ' + %char(dz) + ' '
       + %char(db) + ' [' + q.a + '] ' + %char(q.n) + ' '
       + %char(*in31) + %char(*in32);
say(line);
clear *in;
line = %char(*in32) + ' ' + local();
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lc char(3) inz('zzz');
  dcl-s lp zoned(4:1) inz(3.5);
  dcl-ds ld;
    la char(2) inz('kk');
    ln int(5) inz(4);
  end-ds;
  clear lc;
  clear lp;
  clear ld;
  return '[' + lc + '] ' + %char(lp) + ' [' + la + '] ' + %char(ln);
end-proc;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s wbuf varchar(200);
  wbuf = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :wbuf,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
