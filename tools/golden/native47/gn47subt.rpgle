**FREE
// %SUBST as an EVAL target: a value shorter and longer than the part, a
// varying field, EVALR. Captured on PUB400 (tools/golden/native47.txt
// n47_subt) for the native unit NSUBT.
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('abcdefghij');
dcl-s b char(10) inz('abcdefghij');
dcl-s v varchar(10) inz('abcdef');
dcl-s r char(10) inz('abcdefghij');
%subst(a:3:4) = 'XY';
say('SHORT [' + a + ']');
%subst(b:3:2) = 'WXYZ';
say('LONG [' + b + ']');
%subst(v:2:2) = 'QQ';
say('VARY [' + v + '] ' + %char(%len(v)));
monitor;
  %subst(v:7:1) = 'Z';
  say('VARYPAST [' + v + ']');
on-error;
  say('VARYPAST ERR ' + %char(%status));
endmon;
evalr %subst(r:3:4) = 'XY';
say('EVALR [' + r + ']');
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
