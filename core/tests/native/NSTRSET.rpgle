**FREE
// Native unit NSTRSET (S2.158): %STR as a target, the program captured
// on PUB400 as GN158ST with DSPLY for its output.
// Source: golden capture tools/golden/native158 n158_strset (PUB400,
// IBM i 7.5, 2026-11-20).
ctl-opt dftactgrp(*no);
dcl-s buf char(12) inz(*all'x');
dcl-s small char(4);
dcl-s p pointer;
dcl-s line varchar(80);

p = %addr(buf);
%str(p : 6) = 'Hello world';
line = %str(p) + ' ' + %char(%len(%str(p))) + ' ' + %subst(buf : 7 : 6);
%str(p + 2 : 4) = 'ab';
line += ' ' + %str(p);
%str(%addr(small) : %size(small)) = 'zzzzzz';
line += ' ' + %str(%addr(small));
dsply (line);
*inlr = *on;
