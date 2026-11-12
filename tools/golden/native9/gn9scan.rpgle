**FREE
// Stage 2, S2.12: %SCAN and %XLATE of character expressions, with
// literal and field arguments, and starts at and past the edges.
// Captured on PUB400 (tools/golden/native9.txt n9_scan) for the native
// unit test NSCAN.
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('ABCABCXYZ');
dcl-s b char(6) inz(' aXbYc');
dcl-s c char(10);
dcl-s n packed(3:0);
dcl-s s4 packed(3:0) inz(4);
dcl-s s11 packed(3:0) inz(11);
dcl-s s0 packed(3:0) inz(0);
n = %scan('BC':a);
say('SCAN ' + %char(n));
n = %scan('BC':a:3);
say('SCAN START ' + %char(n));
n = %scan('Q':a);
say('SCAN NONE ' + %char(n));
n = %scan('XYZ ':a);
say('SCAN BLANK ' + %char(n));
n = %scan('A':a) + 10;
say('SCAN EXPR ' + %char(n));
n = %scan('A':a:s4);
say('SCAN VAR ' + %char(n));
n = %scan('C':%trim(b) + a);
say('SCAN CAT ' + %char(n));
monitor;
  n = %scan('A':a:s11);
  say('SCAN PAST ' + %char(n));
on-error;
  say('SCAN PAST ERR ' + %char(%status));
endmon;
monitor;
  n = %scan('A':a:s0);
  say('SCAN ZERO ' + %char(n));
on-error;
  say('SCAN ZERO ERR ' + %char(%status));
endmon;
c = %xlate('abc':'ABC':%trim(b));
say('XLATE [' + c + ']');
c = %xlate('ABC':'xy':a);
say('XLATE SHORT [' + c + ']');
c = %xlate('A':'z':a:2);
say('XLATE START [' + c + ']');
monitor;
  c = %xlate('A':'z':a:s11);
  say('XLATE PAST [' + c + ']');
on-error;
  say('XLATE PAST ERR ' + %char(%status));
endmon;
*inlr = *on;
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
