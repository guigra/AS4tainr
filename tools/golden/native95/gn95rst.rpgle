**FREE
// Stage 2, S2.95: RESET after the values changed: a character and a
// packed field with INZ, a field without INZ, an array with INZ, a data
// structure with INZ subfields, a qualified one, and from a procedure a
// global field and its own local. Captured on PUB400
// (tools/golden/native95.txt n95_reset) for the native unit NRESET.
ctl-opt dftactgrp(*no);
dcl-s c char(4) inz('abcd');
dcl-s p packed(5:2) inz(12.34);
dcl-s z zoned(3:0);
dcl-s arr int(5) dim(3) inz(7);
dcl-ds d;
  dc char(3) inz('QRS');
  dz zoned(3:0) inz(42);
end-ds;
dcl-ds q qualified;
  a char(2) inz('mn');
  n packed(3:0) inz(5);
end-ds;
dcl-s line varchar(120);
c = 'wxyz';
p = 99.5;
z = 8;
arr(2) = 1;
dc = 'TUV';
dz = 1;
q.a = 'op';
q.n = 6;
reset c;
reset p;
reset z;
reset arr;
reset d;
reset q;
line = '[' + c + '] ' + %char(p) + ' ' + %char(z) + ' '
       + %char(arr(1) + arr(2) + arr(3)) + ' [' + dc + '] ' + %char(dz)
       + ' [' + q.a + '] ' + %char(q.n);
say(line);
p = 1;
say(local());
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lc char(3) inz('zzz');
  lc = 'aaa';
  reset lc;
  reset p;
  return '[' + lc + '] ' + %char(p);
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
