**FREE
// Stage 2, S2.19: varying-length character fields. Captured on PUB400
// (tools/golden/native20.txt n20_var) for the native unit test NVAR.
ctl-opt dftactgrp(*no);
dcl-s v varchar(10);
dcl-s w varchar(5) inz('AB');
dcl-s c char(6);
v = 'HELLO';
say('VAR [' + v + '] ' + %char(%len(v)));
v = v + ' W';
say('VAR CAT [' + v + '] ' + %char(%len(v)));
w = 'TOOLONGVALUE';
say('VAR TRUNC [' + w + '] ' + %char(%len(w)));
v = %trim('  X  ');
say('VAR TRIM [' + v + '] ' + %char(%len(v)));
c = w;
say('VAR TO CHAR [' + c + ']');
v = c;
say('VAR FROM CHAR [' + v + '] ' + %char(%len(v)));
if v = 'TOOLO';
  say('VAR CMP PADDED EQ');
else;
  say('VAR CMP PADDED NE');
endif;
v = %subst(c : 1 : 3);
say('VAR SUBST [' + v + '] ' + %char(%len(v)));
if %scan('O' : v) = 2;
  say('VAR SCAN 2');
else;
  say('VAR SCAN OTHER');
endif;
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
