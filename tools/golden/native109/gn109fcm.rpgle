**FREE
// Stage 2, S2.109 (ADR 0008): comparisons with a float operand: a float
// with a literal and with a decimal field (=, <>, >=, <), %FLOAT(1) / 3
// with a float, a DOW over a float, and a procedure's own float.
// Captured on PUB400 (tools/golden/native109.txt n109_floatcmp) for the
// native unit NFLTCMP.
ctl-opt dftactgrp(*no);
dcl-s f8 float(8) inz(1.5);
dcl-s third float(8);
dcl-s p packed(5:2) inz(1.50);
dcl-s k int(5);
dcl-s line varchar(80);
if f8 > 1;
  line += 'a';
endif;
if f8 = p;
  line += 'b';
endif;
if f8 <> p;
  line += 'X';
endif;
third = 1 / 3;
if %float(1) / 3 = third;
  line += 'c';
endif;
if f8 >= 1.5 and f8 < 2;
  line += 'd';
endif;
dow f8 < 100;
  f8 = f8 * 3;
  k += 1;
enddo;
line += %char(k) + ' ' + local();
say(line);
*inlr = *on;

dcl-proc local;
  dcl-pi *n varchar(10);
  end-pi;
  dcl-s lf float(8) inz(-2);
  if lf < 0;
    return 'neg';
  endif;
  return 'pos';
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
