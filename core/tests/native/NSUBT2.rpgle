**FREE
// Native unit NSUBT2: expected values from PUB400, IBM i 7.5 (golden capture
// tools/golden/native47*.txt, cited in native_units.rs).
ctl-opt dftactgrp(*no);
dcl-s a char(10) inz('abcdefghij');
dcl-s b char(10) inz('abcdefghij');
dcl-s v varchar(10) inz('abcdef');
dcl-s r char(10) inz('abcdefghij');
%subst(a:3:4) = 'XY';
dsply ('SHORT [' + a + ']');
%subst(b:3:2) = 'WXYZ';
dsply ('LONG [' + b + ']');
%subst(v:2:2) = 'QQ';
dsply ('VARY [' + v + '] ' + %char(%len(v)));
monitor;
  %subst(v:7:1) = 'Z';
  dsply ('VARYPAST [' + v + ']');
on-error;
  dsply ('VARYPAST ERR ' + %char(%status));
endmon;
evalr %subst(r:3:4) = 'XY';
dsply ('EVALR [' + r + ']');
*inlr = *on;
