**FREE
// Stage 2, S2.18: %REM and %DIV. Captured on PUB400
// (tools/golden/native19.txt n19_rem) for the native unit test NREM.
ctl-opt dftactgrp(*no);
dcl-s i int(10) inz(123);
dcl-s m int(10) inz(-17);
dcl-s z int(10) inz(0);
dcl-s n int(10);
n = %rem(17 : 5);
say('REM ' + %char(n));
n = %rem(m : 5);
say('REM NEG ' + %char(n));
n = %div(17 : 5);
say('DIVF ' + %char(n));
n = %div(m : 5);
say('DIVF NEG ' + %char(n));
n = %rem(i : 7) + %div(i : 7) * 7;
say('REM DIV BACK ' + %char(n));
if %rem(i : 2) = 1;
  say('REM COND ODD');
else;
  say('REM COND EVEN');
endif;
monitor;
  n = %rem(i : z);
  say('REM ZERO ' + %char(n));
on-error;
  say('REM ZERO ERR ' + %char(%status));
endmon;
monitor;
  n = %div(i : z);
  say('DIVF ZERO ' + %char(n));
on-error;
  say('DIVF ZERO ERR ' + %char(%status));
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
