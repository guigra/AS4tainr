**FREE
// Stage 2, S2.32: %ABS of packed, zoned and integer fields, of zero, of an
// expression, in arithmetic and in a condition, and of the smallest int(10)
// stored back into an int(10). Captured on PUB400
// (tools/golden/native41.txt n41_abs) for the native unit test NABS.
ctl-opt dftactgrp(*no);
dcl-s p packed(5:2) inz(-123.45);
dcl-s q packed(5:2) inz(67.89);
dcl-s z zoned(7:3) inz(-0.125);
dcl-s i int(10) inz(-42);
dcl-s m int(10) inz(-2147483648);
dcl-s r packed(9:3);
dcl-s k int(10);
dcl-s u uns(5);
dcl-s w int(20);
r = %abs(p);
say('ABS P ' + %char(r));
r = %abs(q);
say('ABS Q ' + %char(r));
r = %abs(z);
say('ABS Z ' + %char(r));
k = %abs(i);
say('ABS I ' + %char(k));
r = %abs(p - q);
say('ABS DIFF ' + %char(r));
r = %abs(p) + %abs(z) * 2;
say('ABS ARITH ' + %char(r));
r = %abs(0);
say('ABS ZERO ' + %char(r));
u = %abs(i);
say('ABS UNS ' + %char(u));
if %abs(p) > 100;
  say('ABS COND TRUE');
else;
  say('ABS COND FALSE');
endif;
w = %abs(m);
say('ABS MIN INT20 ' + %char(w));
monitor;
  k = %abs(m);
  say('ABS MIN INT10 ' + %char(k));
on-error;
  say('ABS MIN INT10 ERR ' + %char(%status));
endmon;
*inlr = *on;
// Write one line to gn1.out in the job's current directory (DSPLY under
// `system` goes to QSYSOPR, a queue PUB400 users share).
dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
