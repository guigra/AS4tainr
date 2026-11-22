**FREE
// Interpreter fix (fork branch fix/compile-checks): a numeric operand of
// %CONCAT. Expected not to compile (RNF0203). Captured on PUB400
// (tools/golden/native217.txt).
ctl-opt dftactgrp(*no);
dcl-s n packed(3:0) inz(7);
dcl-s s varchar(20);

s = %concat(',' : 'a' : n);
*inlr = *on;
