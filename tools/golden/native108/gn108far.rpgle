**FREE
// Stage 2, S2.108 (ADR 0008): arithmetic with float operands: a float
// times 2, a decimal divided by a float, minus a float, %FLOAT(3) / 7
// into float(4), float results into packed and integer fields, a
// procedure's own float, and a float division by zero under MONITOR.
// Captured on PUB400 (tools/golden/native108.txt n108_floatarith) for
// the native unit NFLTARI.
ctl-opt dftactgrp(*no);
dcl-s f8 float(8) inz(1.5);
dcl-s z8 float(8);
dcl-s f4 float(4);
dcl-s p packed(9:3);
dcl-s n int(10);
dcl-s d packed(5:2) inz(10.5);
dcl-s line varchar(200);
f8 = f8 * 2;
line = %char(f8);
f8 = d / f8;
line += ' ' + %char(f8);
f8 = -f8;
line += ' ' + %char(f8);
f4 = %float(3) / 7;
p = f8 * 10;
n = %float(7) / 2;
line += ' ' + %char(p) + ' ' + %char(n) + ' ' + local();
monitor;
  f8 = f8 / z8;
  line += ' D';
on-error;
  line += ' E ' + %char(%status);
endmon;
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(40);
  end-pi;
  dcl-s lf float(8) inz(4);
  lf = lf / 8 + 1;
  return %char(lf);
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
