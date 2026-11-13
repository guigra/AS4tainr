**FREE
// Stage 2, S2.20: %LEN of varying-length fields. Captured on PUB400
// (tools/golden/native21.txt n21_vlen) for the native unit test NVLEN.
ctl-opt dftactgrp(*no);
dcl-s v varchar(20);
dcl-s e varchar(8);
dcl-s n int(10);
v = 'ABC';
n = %len(v);
say('VLEN ' + %char(n));
v = v + '  ';
n = %len(v) * 2;
say('VLEN EXPR ' + %char(n));
n = %len(e);
say('VLEN EMPTY ' + %char(n));
if %len(v) > 3;
  say('VLEN COND GT');
else;
  say('VLEN COND LE');
endif;
v = %trimr(v);
say('VLEN TRIMR ' + %char(%len(v)));
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
