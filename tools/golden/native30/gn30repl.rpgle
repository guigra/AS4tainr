**FREE
// Stage 2, S2.23: %REPLACE with a start and a length: inside the string,
// an insert (length 0), an append, past the end, start 0. Captured on
// PUB400 (tools/golden/native30.txt n30_repl) for the native unit test
// NREPL.
ctl-opt dftactgrp(*no);
dcl-s s varchar(10) inz('ABCDEFG');
dcl-s x char(2) inz('XY');
dcl-s r varchar(20);
dcl-s p3 int(10) inz(3);
dcl-s p8 int(10) inz(8);
dcl-s p9 int(10) inz(9);
dcl-s p0 int(10) inz(0);
dcl-s l0 int(10) inz(0);
dcl-s l2 int(10) inz(2);
dcl-s l5 int(10) inz(5);
monitor;
  r = %replace(x:s:p3:l2);
  say('REPL START LEN [' + r + ']');
on-error;
  say('REPL START LEN ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p3);
  say('REPL START [' + r + ']');
on-error;
  say('REPL START ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s);
  say('REPL NOSTART [' + r + ']');
on-error;
  say('REPL NOSTART ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p3:l0);
  say('REPL INSERT [' + r + ']');
on-error;
  say('REPL INSERT ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p8:l0);
  say('REPL APPEND [' + r + ']');
on-error;
  say('REPL APPEND ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p3:l5);
  say('REPL LONGLEN [' + r + ']');
on-error;
  say('REPL LONGLEN ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p8:l2);
  say('REPL PASTLEN [' + r + ']');
on-error;
  say('REPL PASTLEN ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p9);
  say('REPL PAST [' + r + ']');
on-error;
  say('REPL PAST ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(x:s:p0);
  say('REPL ZERO [' + r + ']');
on-error;
  say('REPL ZERO ERR ' + %char(%status));
endmon;
monitor;
  r = %replace(%trim('  Q '):s:p3:l2);
  say('REPL TRIM [' + r + ']');
on-error;
  say('REPL TRIM ERR ' + %char(%status));
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
