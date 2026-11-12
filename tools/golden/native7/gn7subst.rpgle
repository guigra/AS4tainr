**FREE
// Stage 2, S2.10: %SUBST of character expressions, with literal and
// field arguments, at the edges and out of range.
// Captured on PUB400 (tools/golden/native7.txt n7_subst) for the native
// unit test NSUB.
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('ABCDEFGHIJ');
dcl-s b char(6) inz('  XYZ ');
dcl-s c char(5);
dcl-s i packed(3:0) inz(2);
dcl-s l int(10) inz(3);
dcl-s s11 packed(3:0) inz(11);
dcl-s s12 packed(3:0) inz(12);
dcl-s s9 packed(3:0) inz(9);
dcl-s l0 packed(3:0) inz(0);
dcl-s l1 packed(3:0) inz(1);
dcl-s l5 packed(3:0) inz(5);
c = %subst(a:3:4);
say('SUBST [' + c + ']');
c = %subst(a:8);
say('SUBST REST [' + c + ']');
c = %subst(a:i:l);
say('SUBST VARS [' + c + ']');
c = %subst(%trim(b):2:2) + 'Q';
say('SUBST TRIM [' + c + ']');
monitor;
  c = %subst(a:s11:l0);
  say('SUBST END ZERO [' + c + ']');
on-error 00100;
  say('SUBST END ZERO 00100');
on-error;
  say('SUBST END ZERO OTHER ' + %char(%status));
endmon;
monitor;
  c = %subst(a:s12:l1);
  say('SUBST START FAIL');
on-error 00100;
  say('SUBST START 00100');
on-error;
  say('SUBST START OTHER ' + %char(%status));
endmon;
monitor;
  c = %subst(a:s9:l5);
  say('SUBST LEN FAIL');
on-error 00100;
  say('SUBST LEN 00100');
on-error;
  say('SUBST LEN OTHER ' + %char(%status));
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
