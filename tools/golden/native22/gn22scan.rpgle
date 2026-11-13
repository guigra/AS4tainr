**FREE
// %SCAN of an empty varying field with a start of 0 or below, for the
// follow-up of upstream PR #9.
ctl-opt dftactgrp(*no);
dcl-s v varchar(10) inz('');
dcl-s n packed(3:0);
dcl-s s0 packed(3:0) inz(0);
dcl-s sm packed(3:0) inz(-5);
dcl-s s99 packed(3:0) inz(99);
monitor;
  n = %scan('A':v:s0);
  say('SCAN EMPTY START0 ' + %char(n));
on-error;
  say('SCAN EMPTY START0 ERR ' + %char(%status));
endmon;
monitor;
  n = %scan('A':v:sm);
  say('SCAN EMPTY STARTNEG ' + %char(n));
on-error;
  say('SCAN EMPTY STARTNEG ERR ' + %char(%status));
endmon;
monitor;
  n = %scan('A':v:s99);
  say('SCAN EMPTY START99 ' + %char(n));
on-error;
  say('SCAN EMPTY START99 ERR ' + %char(%status));
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
