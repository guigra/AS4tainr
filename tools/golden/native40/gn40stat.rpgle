**FREE
// Stage 2, S2.31: %STATUS with no file, in assignments, arithmetic and
// conditions: before any error, after a program error in a MONITOR group,
// and after statements that end normally. The values are kept and
// written at the end: a call to say (SQL inside) would set %STATUS again.
// Captured on PUB400 (tools/golden/native40.txt n40_stat) for the native
// unit test NSTAT.
ctl-opt dftactgrp(*no);
dcl-s n packed(5:0);
dcl-s z packed(5:0) inz(0);
dcl-s a packed(5:0) dim(3);
dcl-s i int(10) inz(4);
dcl-s s varchar(10) inz('abc');
dcl-s c varchar(10);
dcl-s k int(10) dim(9);
dcl-s t varchar(20);
k(1) = %status;
monitor;
  n = 10 / z;
on-error;
  k(2) = %status;
endmon;
k(3) = %status;
n = 5;
k(4) = %status;
monitor;
  a(i) = 1;
on-error;
  k(5) = %status + 1;
endmon;
monitor;
  c = %subst(s:5:1);
on-error;
  if %status = 100;
    t = 'IS 100';
  else;
    t = 'NOT 100';
  endif;
  k(6) = %status * 2;
endmon;
monitor;
  n = 1;
on-error;
  t = 'NONE';
endmon;
k(7) = %status;
monitor;
  n = 10 / z;
on-error;
endmon;
n = 5;
c = 'x';
k(8) = %status;
say('ST START ' + %char(k(1)));
say('ST DIV ' + %char(k(2)));
say('ST AFTER DIV ' + %char(k(3)));
say('ST AFTER EVAL ' + %char(k(4)));
say('ST INDEX PLUS 1 ' + %char(k(5)));
say('ST SUBST ' + t);
say('ST SUBST TIMES 2 ' + %char(k(6)));
say('ST AFTER MONITOR ' + %char(k(7)));
say('ST AFTER TWO EVALS ' + %char(k(8)));
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
