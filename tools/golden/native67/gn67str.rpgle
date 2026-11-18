**FREE
// Stage 2, S2.67: %SCANR, %LEFT and %RIGHT. %SCANR with and without a
// start and a length, not found, a pattern longer than the part searched;
// %LEFT and %RIGHT of a field and of an expression, with lengths 0 and the
// whole string; a length longer than the string, and a %SCANR length past
// the end, under MONITOR. Captured on PUB400 (tools/golden/native67.txt
// n67_str) for the native unit NSCANR.
ctl-opt dftactgrp(*no);
dcl-s s varchar(20) inz('abcabcabc');
dcl-s f char(8) inz('ABCDEFGH');
dcl-s k int(10);
dcl-s n int(10);
k = %scanr('abc' : s);
say('R1 ' + %char(k));
k = %scanr('abc' : s : 2);
say('R2 ' + %char(k));
k = %scanr('abc' : s : 1 : 6);
say('R3 ' + %char(k));
k = %scanr('x' : s);
say('R4 ' + %char(k));
k = %scanr('abcd' : s : 7);
say('R5 ' + %char(k));
k = %scanr('c' : f + s);
say('R6 ' + %char(k));
n = 3;
say('L1 [' + %left(f : n) + '] [' + %right(f : n) + ']');
say('L2 [' + %left(s : 0) + '] [' + %right(f : 8) + ']');
say('L3 [' + %left(%trim(f) + 'xy' : 9) + ']');
monitor;
  n = 9;
  say('L4 ' + %left(f : n));
on-error;
  say('L4 ERR ' + %char(%status));
endmon;
monitor;
  k = %scanr('a' : s : 5 : 10);
  say('R7 ' + %char(k));
on-error;
  say('R7 ERR ' + %char(%status));
endmon;
*inlr = *on;

dcl-proc say;
  dcl-pi *n;
    text varchar(200) const;
  end-pi;
  dcl-s line varchar(200);
  line = text;
  exec sql call qsys2.ifs_write_utf8(path_name => 'gn1.out', line => :line,
                                     overwrite => 'APPEND', end_of_line => 'LF');
end-proc;
